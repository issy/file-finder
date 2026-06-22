use crate::generated::{BaseRule, Rule, StringComparisonBaseRule, StringComparisonRule};
use std::cell::OnceCell;
use std::fs::read_to_string;
use std::ops::Not;
use std::path::PathBuf;
use std::sync::Arc;

struct Context<'a> {
    path: &'a PathBuf,
    relative_to: &'a PathBuf,
    content: OnceCell<Arc<String>>,
}

impl<'a> Context<'a> {
    fn new(path: &'a PathBuf, relative_to: &'a PathBuf) -> Self {
        Context {
            path,
            relative_to,
            content: OnceCell::new(),
        }
    }

    async fn get_content(&self) -> Arc<String> {
        self.content
            .get_or_init(|| Arc::from(read_to_string(&self.path).unwrap().to_string()))
            .clone()
    }
}

impl From<&Rule> for BaseRule {
    fn from(rule: &Rule) -> Self {
        BaseRule {
            dirpath: rule.dirpath.clone(),
            filename: rule.filename.clone(),
            content: rule.content.clone(),
        }
    }
}

fn apply_string_comparison_base_rule(rule: StringComparisonBaseRule, value: String) -> bool {
    match rule {
        StringComparisonBaseRule::Variant0 {
            startswith,
            contains,
            endswith,
        } => {
            startswith.map(|s| value.starts_with(&s)).unwrap_or(true)
                && contains.map(|c| value.contains(&c)).unwrap_or(true)
                && endswith.map(|e| value.ends_with(&e)).unwrap_or(true)
        }
        StringComparisonBaseRule::Variant1 { equals } => value.eq(&equals),
    }
}

async fn apply_string_comparison_rule(rule: StringComparisonRule, value: &String) -> bool {
    match rule {
        StringComparisonRule::Variant0 {
            startswith,
            contains,
            endswith,
            not,
        } => {
            let positive_section = apply_string_comparison_base_rule(
                StringComparisonBaseRule::Variant0 {
                    startswith,
                    contains,
                    endswith,
                },
                value.clone(),
            );
            let negative_section = match not {
                Some(not_rule) => apply_string_comparison_base_rule(not_rule, value.clone()).not(),
                None => true,
            };
            positive_section && negative_section
        }
        StringComparisonRule::Variant1 { equals, not } => {
            let positive_section = apply_string_comparison_base_rule(
                StringComparisonBaseRule::Variant1 { equals },
                value.clone(),
            );
            let negative_section = not
                .map(|not_rule| apply_string_comparison_base_rule(not_rule, value.clone()).not())
                .unwrap_or(true);
            positive_section && negative_section
        }
    }
}

async fn apply_dirpath_rule(rule: StringComparisonRule, ctx: &Context<'_>) -> bool {
    let dirpath = ctx
        .path
        .parent()
        .map(|p| {
            p.strip_prefix(ctx.relative_to)
                .unwrap()
                .to_str()
                .unwrap()
                .to_string()
        })
        .unwrap_or("".into());
    apply_string_comparison_rule(rule, &dirpath).await
}

async fn apply_filename_rule(rule: StringComparisonRule, ctx: &Context<'_>) -> bool {
    let filename = ctx.path.file_name().unwrap().to_str().unwrap().to_string();
    apply_string_comparison_rule(rule, &filename).await
}

async fn apply_content_rule(rule: StringComparisonRule, ctx: &Context<'_>) -> bool {
    let content = ctx.get_content().await;
    apply_string_comparison_rule(rule, &*content).await
}

async fn apply_base_rule(rule: &BaseRule, ctx: &Context<'_>) -> bool {
    let dirpath_result = match rule.dirpath.as_ref() {
        Some(dirpath_rule) => apply_dirpath_rule(dirpath_rule.clone(), ctx).await,
        None => true,
    };

    let filename_result = match rule.filename.as_ref() {
        Some(filename_rule) => apply_filename_rule(filename_rule.clone(), ctx).await,
        None => true,
    };

    let content_result = match rule.content.as_ref() {
        Some(content_rule) => apply_content_rule(content_rule.clone(), ctx).await,
        None => true,
    };

    dirpath_result && filename_result && content_result
}

pub(crate) async fn apply_rule(rule: &Rule, path: &PathBuf, relative_to: &PathBuf) -> bool {
    let ctx = Context::new(path, relative_to);

    let base_result = apply_base_rule(&BaseRule::from(rule), &ctx).await;

    let not_result = match rule.not.as_ref() {
        Some(not_rule) => apply_base_rule(not_rule, &ctx).await.not(),
        None => true,
    };

    base_result && not_result
}
