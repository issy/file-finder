# File Finder

This is a simple CLI tool which allows you to filter through files in the current directory by a nearly infinite set of criteria.

You simply point the tool at a directory and a rules file, and it will return a list of files that match your criteria.

Rules are defined in plain yaml, and can be as simple or complex as you like. You can filter by file name, directory, extension, content, etc.

This tool is still in development, so you may experience bugs. Once I have implemented all the features I plan to add, I will be focusing on performance and clearer error messages.

## Installation

### Homebrew

On macOS and Linux, you can install the tool using Homebrew:

```bash
brew install issy/tap/file-finder
```

### Binary

You can download the latest binary from the [releases page](https://github.com/issy/file-finder/releases)

## Usage

To search for files from the current directory, you can run the following command:

```bash
file-finder rules.yaml
```

To search for files from a different directory, you can run the following command:

```bash
file-finder rules.yaml -d /path/to/directory
```

Your rules file will look something like this

```yaml
rules:
  - filename:
      endswith: Api.ts
    dirpath:
      startswith: src/views
    content:
      contains: "export default"
  - filename:
      endswith: .md
    dirpath:
      startswith: docs
    content:
      contains: "## Usage"
```

Check the examples directory for more examples of rules files. There is also a `schema.json` file which defines the rules schema, and can be used to validate your rules files. This can be useful in IDEs which support JSON schema validation, such as VSCode.
