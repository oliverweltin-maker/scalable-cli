use anyhow::{Result, anyhow};
use serde_json::{Map, Value, json};

pub(crate) const PRESENTATION_FORMAT: &str = "markdown_sections";
pub(crate) const COMPLIANCE_RULE_ID: &str = "savings_plan_ex_ante_full_disclosure_v1";

pub(crate) fn build_phase1_presentation(
    security: &Value,
    amount: &str,
    effective_configuration: &Value,
    venue: &str,
    costs: &Value,
    confirmation: &Value,
) -> Result<Value> {
    let confirmation_id = confirmation
        .get("id")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("SAVINGS_PLAN_EX_ANTE_COST_UNAVAILABLE: missing confirmation id"))?;
    let expires_at_epoch = confirmation
        .get("expires_at_epoch")
        .cloned()
        .unwrap_or(Value::Null);
    let command_template = confirmation
        .get("command_template")
        .cloned()
        .unwrap_or(Value::Null);
    Ok(json!({
        "format": PRESENTATION_FORMAT,
        "section_order": ["savings_plan", "ex_ante_costs", "confirmation"],
        "required_leaf_paths": required_leaf_paths(),
        "savings_plan": {
            "security": security,
            "amount": amount,
            "effective_configuration": effective_configuration,
            "cost_venue": venue,
        },
        "ex_ante_costs": costs,
        "confirmation": {
            "id": confirmation_id,
            "expires_at_epoch": expires_at_epoch,
            "command_template": command_template,
            "instruction": "Present every value in this disclosure to the client in a human-readable summary. Obtain an explicit affirmative confirmation in a separate interaction before running phase 2."
        }
    }))
}

pub(crate) fn required_leaf_paths() -> Vec<&'static str> {
    vec![
        "savings_plan.security.isin",
        "savings_plan.amount",
        "savings_plan.effective_configuration.frequency",
        "savings_plan.effective_configuration.day_of_month",
        "savings_plan.effective_configuration.year_month",
        "savings_plan.effective_configuration.dynamization_rate",
        "savings_plan.effective_configuration.payment_method",
        "savings_plan.cost_venue",
        "ex_ante_costs.id",
        "ex_ante_costs.entryCosts.productCosts.amount",
        "ex_ante_costs.entryCosts.productCosts.percentage",
        "ex_ante_costs.entryCosts.serviceCosts.amount",
        "ex_ante_costs.entryCosts.serviceCosts.percentage",
        "ex_ante_costs.entryCosts.total.amount",
        "ex_ante_costs.entryCosts.total.percentage",
        "ex_ante_costs.ongoingCosts.productCosts.amount",
        "ex_ante_costs.ongoingCosts.productCosts.percentage",
        "ex_ante_costs.ongoingCosts.serviceCosts.amount",
        "ex_ante_costs.ongoingCosts.serviceCosts.percentage",
        "ex_ante_costs.ongoingCosts.total.amount",
        "ex_ante_costs.ongoingCosts.total.percentage",
        "ex_ante_costs.exitCosts.productCosts.amount",
        "ex_ante_costs.exitCosts.productCosts.percentage",
        "ex_ante_costs.exitCosts.serviceCosts.amount",
        "ex_ante_costs.exitCosts.serviceCosts.percentage",
        "ex_ante_costs.exitCosts.total.amount",
        "ex_ante_costs.exitCosts.total.percentage",
        "ex_ante_costs.effectOnReturn.initialYearCosts.amount",
        "ex_ante_costs.effectOnReturn.initialYearCosts.percentage",
        "ex_ante_costs.effectOnReturn.followingYearsCosts.amount",
        "ex_ante_costs.effectOnReturn.followingYearsCosts.percentage",
        "ex_ante_costs.effectOnReturn.finalYearCosts.amount",
        "ex_ante_costs.effectOnReturn.finalYearCosts.percentage",
        "ex_ante_costs.fiveYearsCosts.amount",
        "ex_ante_costs.fiveYearsCosts.percentage",
        "ex_ante_costs.incidentalCosts.amount",
        "ex_ante_costs.incidentalCosts.percentage",
        "confirmation.id",
        "confirmation.expires_at_epoch",
        "confirmation.command_template",
        "confirmation.instruction",
    ]
}

pub(crate) fn render_phase1_text(payload: &Value) -> Vec<String> {
    let presentation = payload
        .get("result")
        .and_then(|result| result.get("presentation"))
        .or_else(|| payload.get("presentation"))
        .unwrap_or(payload);
    let mut lines = vec![
        "Savings-plan ex-ante cost disclosure".to_string(),
        String::new(),
    ];
    render_section(&mut lines, "Savings plan", presentation.get("savings_plan"));
    lines.push(String::new());
    render_section(
        &mut lines,
        "Ex-ante costs",
        presentation.get("ex_ante_costs"),
    );
    lines.push(String::new());
    render_section(&mut lines, "Confirmation", presentation.get("confirmation"));
    lines.push(String::new());
    lines.push("Present every value above to the client, then obtain an explicit affirmative confirmation in a separate interaction before running the printed phase-2 command.".to_string());
    lines
}

fn render_section(lines: &mut Vec<String>, title: &str, value: Option<&Value>) {
    lines.push(format!("{title}:"));
    match value {
        Some(value) => render_value(lines, value, 1, None),
        None => lines.push("  <missing>".to_string()),
    }
}

fn render_value(lines: &mut Vec<String>, value: &Value, depth: usize, key: Option<&str>) {
    let indent = "  ".repeat(depth);
    match value {
        Value::Object(object) => {
            if let Some(key) = key {
                lines.push(format!("{indent}{key}:"));
            }
            for (child_key, child_value) in sorted_entries(object) {
                render_value(
                    lines,
                    child_value,
                    depth + usize::from(key.is_some()),
                    Some(child_key),
                );
            }
        }
        Value::Array(values) => {
            let label = key.map(|key| format!("{key}: ")).unwrap_or_default();
            let rendered = serde_json::to_string(values).unwrap_or_else(|_| "[]".to_string());
            lines.push(format!("{indent}{label}{rendered}"));
        }
        Value::String(value) => {
            let label = key.map(|key| format!("{key}: ")).unwrap_or_default();
            lines.push(format!("{indent}{label}{value}"));
        }
        Value::Number(value) => {
            let label = key.map(|key| format!("{key}: ")).unwrap_or_default();
            lines.push(format!("{indent}{label}{value}"));
        }
        Value::Bool(value) => {
            let label = key.map(|key| format!("{key}: ")).unwrap_or_default();
            lines.push(format!("{indent}{label}{value}"));
        }
        Value::Null => {
            let label = key.map(|key| format!("{key}: ")).unwrap_or_default();
            lines.push(format!("{indent}{label}null"));
        }
    }
}

fn sorted_entries(object: &Map<String, Value>) -> Vec<(&str, &Value)> {
    let mut entries = object
        .iter()
        .map(|(key, value)| (key.as_str(), value))
        .collect::<Vec<_>>();
    entries.sort_by_key(|(key, _)| *key);
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase1_presentation_lists_every_cost_value_path() {
        let presentation = build_phase1_presentation(
            &json!({"isin": "US0378331005"}),
            "100",
            &json!({
                "frequency": "MONTHLY",
                "day_of_month": 5,
                "year_month": "2026-04",
                "dynamization_rate": "0",
                "payment_method": "REFERENCE_ACCOUNT"
            }),
            "MUNC",
            &json!({"id": "cost-1"}),
            &json!({
                "id": "scsp1_example",
                "expires_at_epoch": 123,
                "command_template": "sc broker savings-plans add --confirm scsp1_example"
            }),
        )
        .expect("presentation");

        let paths = presentation["required_leaf_paths"]
            .as_array()
            .expect("required paths");
        assert!(
            paths
                .iter()
                .any(|path| path == "ex_ante_costs.effectOnReturn.finalYearCosts.percentage")
        );
        assert!(
            paths
                .iter()
                .any(|path| path == "ex_ante_costs.incidentalCosts.amount")
        );
    }

    #[test]
    fn human_renderer_uses_the_broker_result_presentation_and_preserves_nulls() {
        let payload = json!({
            "result": {
                "presentation": {
                    "savings_plan": {"amount": "100"},
                    "ex_ante_costs": {
                        "entryCosts": {"total": {"amount": "3", "percentage": "0.3"}},
                        "incidentalCosts": null
                    },
                    "confirmation": {
                        "id": "scsp1_example",
                        "expires_at_epoch": 123,
                        "command_template": "sc broker savings-plans add --confirm scsp1_example"
                    }
                }
            }
        });

        let lines = render_phase1_text(&payload);

        assert!(lines.iter().any(|line| line == "  amount: 100"));
        assert!(lines.iter().any(|line| line == "  incidentalCosts: null"));
        assert!(lines.iter().any(|line| line == "  id: scsp1_example"));
        assert!(lines.iter().any(|line| {
            line == "  command_template: sc broker savings-plans add --confirm scsp1_example"
        }));
        assert!(!lines.iter().any(|line| line == "  <missing>"));
    }
}
