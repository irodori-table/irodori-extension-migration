<!-- i18n: language-switcher -->
[English](README.md) | [日本語](README.ja.md)

# Migration Studio

An [Irodori Table](https://github.com/irodori-table/irodori-table) extension that plans a table migration between database engines. Install it from the extension store, then open it from **Tools → Migration Studio** or the command palette.

From a source and target table it produces source and target SQL, a data diff, and a runbook, with the validation gates and engine notes for that pair. It only writes text: nothing connects to a database, and you run the SQL yourself.

Planning lives in the [`irodori-migration`](https://github.com/irodori-table/irodori-migration) crate. This extension maps the host's form onto it and the plan onto output tabs.

## How it plugs in

The extension is a native module that contributes a *tool*. `irodori.extension.json` describes the form under `contributes.tools`, and the host renders it, so no extension code runs in the application webview. Each change sends a `tool.run` call, handled in `src/driver.rs`, and the response is the tabs to show. The host runs the module in a separate process, so a crash here cannot take the app down.

## Development

```sh
make check    # fmt, clippy, tests
make package  # release build and dist/irodori-extension-migration.tar.gz
```

## License

0BSD or MIT, at your option.
