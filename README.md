# amalgamation-rs

Amalgamation-rs is a tiny command-line tool that downloads a GitHub repository and merges all text/source files into a single file for easier browsing and LLM analysis.

The rationale of this app is that I recent found that I want to use LLM to assist coding as well as quick browsing the
code, it turns out it would be a great idea if I can just input a url of a github repo and then merge all the source

## Usage

Basic merge mode:
    run amalgamator <url> -o <output-name>
with `-v` or `--verbose` to show more logs
Interactive mode:
    run amalgamator <url> -o <output-name> --interactive
This would open a TUI menu, and then you can select/unselect the files/folders, hit `tab` to switch focused panel, and then hit enter to confirm