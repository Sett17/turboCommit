# turbocommit

turbocommit is a CLI tool written in Rust that generates commit messages in accordance with the [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) specification. It uses the git diff to create commit messages that accurately reflect the changes made to a repository.

## Installation

turbocommit can be easily installed with Cargo, Rust's package manager. Simply run the following command:

```bash
cargo install turbocommit
```

Please note that in order to use turbocommit, you will need to set the `OPENAI_API_KEY` environment variable. This API key is required to use the OpenAI `gpt-3.5-turbo` language model, which is used by turbocommit to generate commit messages.

## Usage

Using turbocommit is simple. Once it is installed and your `OPENAI_API_KEY` is set, navigate to your repository and run the following command:

```bash
turbocommit
```

This will generate a commit message based on the changes made to the repository, following the Conventional Commits specification.

Using turbocommit can help you keep your git commit history at a higher quality, as it generates informative and standardized commit messages that accurately reflect the changes made to the repository. Additionally, because it uses OpenAI's `gpt-3.5-turbo` language model, it is a very cheap way to improve the quality of your git commit messages.
