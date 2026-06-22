use crate::generated::{
    BaseRule, BaseRuleCombinator, NumberComparisonBaseRule, Rule, RuleCombinator,
    StringComparisonBaseRule,
};
use futures::{StreamExt, stream};
use std::cell::OnceCell;
use std::fs::read_to_string;
use std::ops::Not;
use std::path::PathBuf;
use std::sync::Arc;

pub(crate) struct Context<'a> {
    path: &'a PathBuf,
    relative_to: &'a PathBuf,
    content: OnceCell<Arc<String>>,
}

impl<'a> Context<'a> {
    pub(crate) fn new(path: &'a PathBuf, relative_to: &'a PathBuf) -> Self {
        Context {
            path,
            relative_to,
            content: OnceCell::new(),
        }
    }

    async fn get_content(&self) -> &String {
        self.content
            .get_or_init(|| Arc::from(read_to_string(self.path).unwrap().to_string()))
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

async fn apply_dirpath_rule(rule: StringComparisonBaseRule, ctx: &Context<'_>) -> bool {
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
        .map(|path| if path.is_empty() { path } else { path + "/" })
        .unwrap_or("".into());
    apply_string_comparison_base_rule(rule, dirpath)
}

async fn apply_filename_rule(rule: StringComparisonBaseRule, ctx: &Context<'_>) -> bool {
    let filename = ctx.path.file_name().unwrap().to_str().unwrap().to_string();
    apply_string_comparison_base_rule(rule, filename)
}

async fn apply_content_rule(rule: StringComparisonBaseRule, ctx: &Context<'_>) -> bool {
    let content = ctx.get_content().await;
    apply_string_comparison_base_rule(rule, content.clone())
}

async fn apply_number_of_lines_rule(rule: NumberComparisonBaseRule, ctx: &Context<'_>) -> bool {
    let content = ctx.get_content().await;
    let number_of_lines = content.lines().count() as i64;
    match rule {
        NumberComparisonBaseRule::LessThan(less_than) => number_of_lines < less_than,
        NumberComparisonBaseRule::GreaterThan(greater_than) => number_of_lines > greater_than,
        NumberComparisonBaseRule::EqualTo(equal_to) => number_of_lines == equal_to,
    }
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

    let number_of_lines_result = match rule.number_of_lines.as_ref() {
        Some(number_of_lines_rule) => {
            apply_number_of_lines_rule(number_of_lines_rule.clone(), ctx).await
        }
        None => true,
    };

    dirpath_result && filename_result && content_result && number_of_lines_result
}

async fn apply_base_rules(rule_combinator: BaseRuleCombinator, ctx: &Context<'_>) -> bool {
    match rule_combinator {
        BaseRuleCombinator::Variant0 { or } => {
            stream::iter(or)
                .map(|rule| async move { apply_base_rule(&rule, ctx).await })
                .buffer_unordered(32)
                .any(|r| async move { r })
                .await
        }
        BaseRuleCombinator::Variant1 { xor } => stream::iter(xor)
            .map(|rule| async move { apply_base_rule(&rule, ctx).await })
            .buffer_unordered(32)
            .filter(|r| futures::future::ready(*r))
            .count()
            .await
            .eq(&1),
        BaseRuleCombinator::Variant2 { and } => {
            stream::iter(and)
                .map(|rule| async move { apply_base_rule(&rule, ctx).await })
                .buffer_unordered(32)
                .all(|r| async move { r })
                .await
        }
    }
}

pub(crate) async fn apply_rule(rule: &Rule, ctx: &Context<'_>) -> bool {
    match rule {
        Rule::Variant0 {
            filename,
            dirpath,
            content,
            not,
            number_of_lines,
        } => {
            let base_rule = BaseRule {
                dirpath: dirpath.clone(),
                content: content.clone(),
                filename: filename.clone(),
                number_of_lines: number_of_lines.clone(),
            };
            let base_result = apply_base_rule(&base_rule, ctx).await;
            let not_result = match not.as_ref() {
                Some(not_rule) => apply_base_rule(not_rule, ctx).await.not(),
                None => true,
            };
            base_result && not_result
        }
        Rule::Variant1(base_rule_combinator) => {
            apply_base_rules(base_rule_combinator.clone(), ctx).await
        }
    }
}

pub(crate) async fn apply_rules(rule_combinator: &RuleCombinator, ctx: &Context<'_>) -> bool {
    match rule_combinator {
        RuleCombinator::Variant0 { or } => {
            stream::iter(or)
                .map(|rule| async move { apply_rule(rule, ctx).await })
                .buffer_unordered(32)
                .any(|r| async move { r })
                .await
        }
        RuleCombinator::Variant1 { xor } => stream::iter(xor)
            .map(|rule| async move { apply_rule(rule, ctx).await })
            .buffer_unordered(32)
            .filter(|r| futures::future::ready(*r))
            .count()
            .await
            .eq(&1),
        RuleCombinator::Variant2 { and } => {
            stream::iter(and)
                .map(|rule| async move { apply_rule(rule, ctx).await })
                .buffer_unordered(32)
                .all(|r| async move { r })
                .await
        }
    }
}
