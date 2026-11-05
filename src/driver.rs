use irodori_migration::{
    parse_column_list, try_build_migration_plan, MigrationEngine, MigrationExportFormat,
    MigrationPlan, MigrationSpec, MigrationTaskLevel,
};
use serde_json::{json, Map, Value};

use crate::abi::{self, IrodoriConnectorBuffer};
use crate::text::{self, Text};
use crate::{ABI_VERSION, CONFIG_JSON, DRIVER_LINKED, ENGINE, MANIFEST_JSON};

const TOOL_ID: &str = "migration.studio";
const NULL_TOKEN: &str = "__IRODORI_NULL__";
const DEFAULT_DELIMITER: &str = "|#|";
const HASH_LABEL: &str = "MD5";

pub fn call_json(request: IrodoriConnectorBuffer) -> IrodoriConnectorBuffer {
    let request = match abi::parse_request(request) {
        Ok(request) => request,
        Err(response) => return response,
    };
    let method = match abi::request_method(request.as_ref()) {
        Ok(method) => method,
        Err(response) => return response,
    };

    match method {
        "health" | "ping" => abi::ok(identity()),
        "describe" | "capabilities" => {
            let mut response = identity();
            for (key, source) in [("manifest", MANIFEST_JSON), ("config", CONFIG_JSON)] {
                response.insert(
                    key.to_string(),
                    serde_json::from_str(source).unwrap_or(Value::Null),
                );
            }
            abi::ok(response)
        }
        "manifest" => abi::owned_buffer(MANIFEST_JSON.to_string()),
        "config" => abi::owned_buffer(CONFIG_JSON.to_string()),
        "tool.run" => match run_tool(request.as_ref().unwrap_or(&Value::Null)) {
            Ok(output) => abi::ok(output),
            Err(message) => abi::error("tool.invalidInput", message),
        },
        other => abi::error(
            "tool.unknownMethod",
            format!("unknown tool method: {other}"),
        ),
    }
}

fn identity() -> Map<String, Value> {
    Map::from_iter([
        ("engine".to_string(), Value::String(ENGINE.to_string())),
        ("abiVersion".to_string(), json!(ABI_VERSION)),
        ("driverLinked".to_string(), Value::Bool(DRIVER_LINKED)),
    ])
}

fn run_tool(request: &Value) -> Result<Map<String, Value>, String> {
    let tool = request.get("tool").and_then(Value::as_str);
    if tool != Some(TOOL_ID) {
        return Err(format!("unknown tool: {}", tool.unwrap_or("<none>")));
    }
    let input = request.get("input").unwrap_or(&Value::Null);
    let locale = request
        .get("locale")
        .and_then(Value::as_str)
        .unwrap_or("en");
    let plan = try_build_migration_plan(&spec_from_input(input)?).map_err(|e| e.to_string())?;
    Ok(output_from_plan(plan, text::for_locale(locale)))
}

fn spec_from_input(input: &Value) -> Result<MigrationSpec, String> {
    let defaults = MigrationSpec::default();
    let delimiter = text_field(input, "delimiter");
    Ok(MigrationSpec {
        source_engine: engine(text_field(input, "sourceEngine"))?,
        target_engine: engine(text_field(input, "targetEngine"))?,
        source_version: text_field(input, "sourceVersion").to_string(),
        target_version: text_field(input, "targetVersion").to_string(),
        source_table: text_field(input, "sourceTable").to_string(),
        target_table: text_field(input, "targetTable").to_string(),
        key_columns: parse_column_list(text_field(input, "keyColumns")),
        compare_columns: parse_column_list(text_field(input, "compareColumns")),
        partition_column: text_field(input, "partitionColumn").to_string(),
        partition_predicate: text_field(input, "partitionPredicate").to_string(),
        export_format: export_format(text_field(input, "exportFormat"))?,
        batch_size: clamped(input, "batchSize", 1_000, 100_000_000),
        diff_limit: clamped(input, "diffLimit", 10, 100_000) as usize,
        hash_bucket_prefix_len: defaults.hash_bucket_prefix_len,
        null_token: NULL_TOKEN.to_string(),
        delimiter: if delimiter.is_empty() {
            DEFAULT_DELIMITER.to_string()
        } else {
            delimiter.to_string()
        },
        normalize_whitespace: flag(input, "normalizeWhitespace"),
        normalize_case: flag(input, "normalizeCase"),
    })
}

fn text_field<'a>(input: &'a Value, field: &str) -> &'a str {
    input.get(field).and_then(Value::as_str).unwrap_or_default()
}

fn flag(input: &Value, field: &str) -> bool {
    input.get(field).and_then(Value::as_bool).unwrap_or(false)
}

/// A value the form could not make a number, such as an emptied box, takes the
/// minimum rather than failing the whole plan while someone is still typing.
fn clamped(input: &Value, field: &str, min: u64, max: u64) -> u64 {
    let value = input.get(field).and_then(|value| {
        value
            .as_f64()
            .or_else(|| value.as_str().and_then(|text| text.trim().parse().ok()))
    });
    match value {
        Some(number) if number.is_finite() => (number.round().max(0.0) as u64).clamp(min, max),
        _ => min,
    }
}

fn engine(id: &str) -> Result<MigrationEngine, String> {
    Ok(match id {
        "hive" => MigrationEngine::Hive,
        "snowflake" => MigrationEngine::Snowflake,
        "duckdb" => MigrationEngine::DuckDb,
        "iceberg" => MigrationEngine::Iceberg,
        "s3Tables" => MigrationEngine::S3Tables,
        "postgres" => MigrationEngine::Postgres,
        "oracle" => MigrationEngine::Oracle,
        "mysql" => MigrationEngine::MySql,
        "mariadb" => MigrationEngine::MariaDb,
        "redshift" => MigrationEngine::Redshift,
        "databricks" => MigrationEngine::Databricks,
        "trinoPresto" => MigrationEngine::TrinoPresto,
        other => return Err(format!("unsupported engine: {other}")),
    })
}

fn export_format(id: &str) -> Result<MigrationExportFormat, String> {
    Ok(match id {
        "parquet" => MigrationExportFormat::Parquet,
        "csv" => MigrationExportFormat::Csv,
        "tsv" => MigrationExportFormat::Tsv,
        other => return Err(format!("unsupported export format: {other}")),
    })
}

fn output_from_plan(plan: MigrationPlan, text: &Text) -> Map<String, Value> {
    let steps: Vec<Value> = plan
        .tasks
        .iter()
        .map(|task| {
            json!({
                "title": task.title,
                "detail": task.detail,
                "level": match task.level {
                    MigrationTaskLevel::Ready => "ready",
                    MigrationTaskLevel::Manual => "manual",
                    MigrationTaskLevel::Risk => "risk",
                },
            })
        })
        .collect();

    let mut notes = vec![json!({ "title": text.engine_notes, "items": plan.pair_notes })];
    if !plan.warnings.is_empty() {
        notes.push(json!({
            "title": text.warnings,
            "items": plan.warnings,
            "tone": "warning",
        }));
    }

    let overview = json!({
        "id": "overview",
        "title": text.overview,
        "kind": "overview",
        "text": overview_text(&plan, text),
        "metrics": [
            {
                "label": text.route,
                "value": format!("{} -> {}", plan.source_label, plan.target_label),
            },
            { "label": text.keys, "value": plan.keys.len().to_string() },
            { "label": text.hash_columns, "value": plan.hash_columns.len().to_string() },
            { "label": text.hash, "value": HASH_LABEL },
            { "label": text.warnings, "value": plan.warnings.len().to_string() },
        ],
        "steps": steps,
        "notes": notes,
    });

    Map::from_iter([
        ("title".to_string(), Value::String(plan.title)),
        (
            "tabs".to_string(),
            json!([
                overview,
                { "id": "source", "title": text.source_sql, "kind": "code", "text": plan.source_sql },
                { "id": "target", "title": text.target_sql, "kind": "code", "text": plan.target_sql },
                { "id": "diff", "title": text.diff, "kind": "code", "text": plan.diff_sql },
                { "id": "runbook", "title": text.runbook, "kind": "text", "text": plan.runbook },
            ]),
        ),
    ])
}

/// What Copy and "put in editor" use for the overview.
fn overview_text(plan: &MigrationPlan, text: &Text) -> String {
    fn bullets(items: &[String], empty: &str) -> Vec<String> {
        if items.is_empty() {
            return vec![format!("- {empty}")];
        }
        items.iter().map(|item| format!("- {item}")).collect()
    }

    let mut lines = vec![
        format!("# {}", plan.title),
        String::new(),
        format!("## {}", text.validation_gates),
    ];
    lines.extend(
        plan.tasks
            .iter()
            .map(|task| format!("- {}: {}", task.title, task.detail)),
    );
    lines.extend([String::new(), format!("## {}", text.engine_notes)]);
    lines.extend(bullets(&plan.pair_notes, text.no_engine_notes));
    lines.extend([String::new(), format!("## {}", text.warnings)]);
    lines.extend(bullets(&plan.warnings, text.no_blocking_warning));
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> Value {
        serde_json::from_str(MANIFEST_JSON).expect("manifest")
    }

    /// The input the host sends for an untouched form, from the manifest's own
    /// defaults, so a form that drifts from the driver fails here.
    fn form_defaults() -> Value {
        let manifest = manifest();
        let mut input = Map::new();
        for section in manifest["contributes"]["tools"][0]["form"]["sections"]
            .as_array()
            .expect("sections")
        {
            for field in section["fields"].as_array().expect("fields") {
                input.insert(
                    field["id"].as_str().expect("id").to_string(),
                    field["default"].clone(),
                );
            }
        }
        Value::Object(input)
    }

    fn run(input: Value, locale: &str) -> Result<Map<String, Value>, String> {
        run_tool(&json!({ "tool": TOOL_ID, "input": input, "locale": locale }))
    }

    fn tab<'a>(output: &'a Map<String, Value>, id: &str) -> &'a Value {
        output["tabs"]
            .as_array()
            .expect("tabs")
            .iter()
            .find(|tab| tab["id"] == id)
            .expect("tab")
    }

    #[test]
    fn the_tool_in_the_manifest_is_the_one_the_driver_answers() {
        assert_eq!(manifest()["contributes"]["tools"][0]["id"], TOOL_ID);
    }

    #[test]
    fn the_extension_id_is_what_the_abi_reports() {
        assert_eq!(manifest()["id"], ENGINE);
        let config: Value = serde_json::from_str(CONFIG_JSON).expect("config");
        assert_eq!(config["extensionId"], ENGINE);
    }

    #[test]
    fn an_untouched_form_produces_every_tab() {
        let output = run(form_defaults(), "en").expect("plan");

        assert_eq!(
            output["title"].as_str().expect("title"),
            "Apache Hive Hive 2/3 -> Snowflake Snowflake"
        );
        let ids: Vec<_> = output["tabs"]
            .as_array()
            .expect("tabs")
            .iter()
            .map(|tab| tab["id"].as_str().expect("id"))
            .collect();
        assert_eq!(ids, ["overview", "source", "target", "diff", "runbook"]);
        for id in ["source", "target", "diff", "runbook"] {
            assert!(!tab(&output, id)["text"].as_str().expect("text").is_empty());
        }
    }

    #[test]
    fn the_overview_counts_what_the_plan_holds() {
        let output = run(form_defaults(), "en").expect("plan");
        let metric = |label: &str| {
            tab(&output, "overview")["metrics"]
                .as_array()
                .expect("metrics")
                .iter()
                .find(|metric| metric["label"] == label)
                .expect("metric")["value"]
                .clone()
        };

        assert_eq!(metric("Keys"), "2");
        assert_eq!(metric("Hash columns"), "6");
        assert_eq!(metric("Hash"), "MD5");
        let text = tab(&output, "overview")["text"].as_str().expect("text");
        assert!(text.contains("## Validation Gates"), "{text}");
    }

    #[test]
    fn output_labels_follow_the_locale() {
        let output = run(form_defaults(), "ja").expect("plan");

        assert_eq!(tab(&output, "source")["title"], "ソース SQL");
        assert!(tab(&output, "overview")["text"]
            .as_str()
            .expect("text")
            .contains("## 検証ゲート"));
    }

    #[test]
    fn numbers_are_clamped_and_an_emptied_box_takes_the_minimum() {
        for (value, expected) in [
            (json!(5), 1_000),
            (json!(2_000_000_000u64), 100_000_000),
            (json!("5000"), 5_000),
            (json!(""), 1_000),
            (Value::Null, 1_000),
        ] {
            let input = json!({ "batchSize": value });
            assert_eq!(clamped(&input, "batchSize", 1_000, 100_000_000), expected);
        }
    }

    #[test]
    fn an_empty_delimiter_falls_back() {
        let mut input = form_defaults();
        input["delimiter"] = json!("");

        assert_eq!(
            spec_from_input(&input).expect("spec").delimiter,
            DEFAULT_DELIMITER
        );
    }

    #[test]
    fn every_option_the_form_offers_is_accepted() {
        let manifest = manifest();
        let fields = manifest["contributes"]["tools"][0]["form"]["sections"]
            .as_array()
            .expect("sections")
            .iter()
            .flat_map(|section| section["fields"].as_array().expect("fields"));
        for field in fields {
            for option in field["options"].as_array().into_iter().flatten() {
                let value = option["value"].as_str().expect("value");
                match field["id"].as_str().expect("id") {
                    "sourceEngine" | "targetEngine" => engine(value).map(|_| ()).expect(value),
                    "exportFormat" => export_format(value).map(|_| ()).expect(value),
                    other => panic!("no mapping for options of {other}"),
                }
            }
        }
    }

    #[test]
    fn a_plan_the_planner_refuses_is_an_error_not_a_panic() {
        let mut input = form_defaults();
        input["keyColumns"] = json!("");
        input["compareColumns"] = json!("");

        let error = run(input, "en").expect_err("no keys");
        assert!(!error.is_empty());
    }

    #[test]
    fn another_tool_and_an_unknown_engine_are_refused() {
        assert!(run_tool(&json!({ "tool": "other", "input": {} })).is_err());
        let mut input = form_defaults();
        input["sourceEngine"] = json!("nonsense");
        assert!(run(input, "en").unwrap_err().contains("nonsense"));
    }
}
