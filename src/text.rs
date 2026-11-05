//! Everything the output says that does not come from the planner.

pub(crate) struct Text {
    pub overview: &'static str,
    pub source_sql: &'static str,
    pub target_sql: &'static str,
    pub diff: &'static str,
    pub runbook: &'static str,
    pub route: &'static str,
    pub keys: &'static str,
    pub hash_columns: &'static str,
    pub hash: &'static str,
    pub warnings: &'static str,
    pub validation_gates: &'static str,
    pub engine_notes: &'static str,
    pub no_engine_notes: &'static str,
    pub no_blocking_warning: &'static str,
}

const EN: Text = Text {
    overview: "Overview",
    source_sql: "Source SQL",
    target_sql: "Target SQL",
    diff: "Diff",
    runbook: "Runbook",
    route: "Route",
    keys: "Keys",
    hash_columns: "Hash columns",
    hash: "Hash",
    warnings: "Warnings",
    validation_gates: "Validation Gates",
    engine_notes: "Engine Notes",
    no_engine_notes: "No engine-specific notes were generated for this plan.",
    no_blocking_warning: "No blocking warnings were generated.",
};

const JA: Text = Text {
    overview: "概要",
    source_sql: "ソース SQL",
    target_sql: "ターゲット SQL",
    diff: "差分",
    runbook: "Runbook",
    route: "ルート",
    keys: "キー",
    hash_columns: "ハッシュ列",
    hash: "ハッシュ",
    warnings: "警告",
    validation_gates: "検証ゲート",
    engine_notes: "エンジン注記",
    no_engine_notes: "この計画にはエンジン固有の注記は生成されませんでした。",
    no_blocking_warning: "ブロックする警告は生成されませんでした。",
};

pub(crate) fn for_locale(locale: &str) -> &'static Text {
    if locale.starts_with("ja") {
        &JA
    } else {
        &EN
    }
}
