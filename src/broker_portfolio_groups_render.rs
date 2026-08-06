use serde_json::Value;

pub(crate) fn render_broker_portfolio_groups_text(
    payload: &Value,
    filtered_to_group: bool,
) -> Vec<String> {
    let result = payload.get("result").unwrap_or(payload);
    let groups = result
        .get("portfolio_groups")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let ungrouped_items = result
        .get("ungrouped_items")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);

    let mut lines = vec![
        format!("groups: {}", groups.len()),
        format!(
            "offer_allows_additional_group: {}",
            display_value(result.get("offer_allows_additional_group"))
        ),
        format!(
            "max_groups_per_portfolio_reached: {}",
            display_value(result.get("max_groups_per_portfolio_reached"))
        ),
    ];

    for group in groups {
        lines.push(String::new());
        lines.push(format!(
            "Group: {} ({})",
            display_value(group.get("name")),
            display_value(group.get("group_id"))
        ));
        lines.push(format!(
            "description: {}",
            display_value(group.get("description"))
        ));
        lines.push(format!(
            "items: {}",
            group
                .get("items")
                .and_then(Value::as_array)
                .map(Vec::len)
                .unwrap_or(0)
        ));
        lines.push(format!(
            "pending_orders: {}",
            display_value(group.get("number_of_pending_orders"))
        ));
        lines.push(format!(
            "savings_plans_amount: {}",
            display_value(group.get("savings_plans_amount"))
        ));

        let performance = group.get("performance").filter(|value| !value.is_null());
        let currency = performance.and_then(|value| value.get("currency").and_then(Value::as_str));
        lines.push(format!(
            "valuation: {}",
            display_money(
                performance.and_then(|value| value.get("valuation")),
                currency
            )
        ));
        let since_buy = performance.and_then(|value| value.get("since_buy"));
        lines.push(format!(
            "since_buy_performance: {}",
            display_value(since_buy.and_then(|value| value.get("performance")))
        ));
        lines.push(format!(
            "since_buy_simple_absolute_return: {}",
            display_money(
                since_buy.and_then(|value| value.get("simple_absolute_return")),
                currency,
            )
        ));
        lines.push("holdings:".to_string());
        for item in group
            .get("items")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or(&[])
        {
            lines.push(format!("  - {}", format_item(item)));
        }
    }

    if !filtered_to_group && !ungrouped_items.is_empty() {
        lines.push(String::new());
        lines.push("Ungrouped".to_string());
        lines.push(format!("items: {}", ungrouped_items.len()));
        for item in ungrouped_items {
            lines.push(format!("  - {}", format_item(item)));
        }
    }

    lines
}

pub(crate) fn render_broker_portfolio_groups_mutation_text(payload: &Value) -> Vec<String> {
    let result = payload.get("result").unwrap_or(payload);
    let action = display_value(result.get("action"));
    let mut lines = vec![format!("action: {action}")];

    if let Some(group_id) = result.get("group_id") {
        lines.push(format!("group_id: {}", display_value(Some(group_id))));
    }
    if let Some(name) = result.get("name") {
        lines.push(format!("name: {}", display_value(Some(name))));
    }
    if result.get("description").is_some() {
        lines.push(format!(
            "description: {}",
            display_value(result.get("description"))
        ));
    }
    if let Some(isins) = result.get("isins").and_then(Value::as_array) {
        lines.push(format!(
            "isins: {}",
            isins
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    if let Some(state) = result.get("portfolio_groups_state") {
        lines.push(String::new());
        lines.extend(render_broker_portfolio_groups_text(state, false));
    }

    lines
}

fn format_item(item: &Value) -> String {
    let name = display_value(item.get("name"));
    let isin = display_value(item.get("isin"));
    let security_type = item.get("security_type").and_then(Value::as_str);
    match security_type.filter(|value| !value.is_empty()) {
        Some(security_type) => format!("{name} ({isin}, {security_type})"),
        None => format!("{name} ({isin})"),
    }
}

fn display_value(value: Option<&Value>) -> String {
    match value.unwrap_or(&Value::Null) {
        Value::Null => "<none>".to_string(),
        Value::Bool(raw) => raw.to_string(),
        Value::String(raw) => raw.clone(),
        other => other.to_string(),
    }
}

fn display_money(value: Option<&Value>, currency: Option<&str>) -> String {
    match value.unwrap_or(&Value::Null) {
        Value::Null => "<none>".to_string(),
        Value::String(raw) => match currency.filter(|value| !value.is_empty()) {
            Some(currency) => format!("{raw} {currency}"),
            None => raw.clone(),
        },
        other => match currency.filter(|value| !value.is_empty()) {
            Some(currency) => format!("{other} {currency}"),
            None => other.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        render_broker_portfolio_groups_mutation_text, render_broker_portfolio_groups_text,
    };

    fn sample_payload() -> serde_json::Value {
        json!({
            "result": {
                "offer_allows_additional_group": true,
                "max_groups_per_portfolio_reached": false,
                "portfolio_groups": [
                    {
                        "group_id": "group-1",
                        "name": "AI portfolio",
                        "description": "tracked by agent",
                        "number_of_pending_orders": 1,
                        "savings_plans_amount": "25.00",
                        "performance": {
                            "valuation": "1234.56",
                            "currency": "EUR",
                            "since_buy": {
                                "performance": "0.12",
                                "simple_absolute_return": "132.45"
                            }
                        },
                        "items": [
                            {
                                "isin": "US0378331005",
                                "name": "Apple Inc.",
                                "security_type": "STOCK"
                            }
                        ]
                    }
                ],
                "ungrouped_items": [
                    {
                        "isin": "IE00B4ND3602",
                        "name": "MSCI World ETF",
                        "security_type": "ETF"
                    }
                ]
            }
        })
    }

    #[test]
    fn render_portfolio_groups_text_supports_wrapped_payloads() {
        let lines = render_broker_portfolio_groups_text(&sample_payload(), false);

        assert!(lines.contains(&"groups: 1".to_string()));
        assert!(lines.contains(&"Group: AI portfolio (group-1)".to_string()));
        assert!(lines.contains(&"since_buy_simple_absolute_return: 132.45 EUR".to_string()));
        assert!(lines.contains(&"Ungrouped".to_string()));
    }

    #[test]
    fn render_portfolio_groups_text_omits_ungrouped_block_when_filtered() {
        let lines = render_broker_portfolio_groups_text(&sample_payload(), true);

        assert!(!lines.iter().any(|line| line == "Ungrouped"));
    }

    #[test]
    fn render_portfolio_groups_mutation_includes_action_and_readback_state() {
        let payload = json!({
            "result": {
                "action": "create",
                "group_id": "group-1",
                "name": "AI portfolio",
                "description": null,
                "isins": ["US0378331005", "IE00B4ND3602"],
                "portfolio_groups_state": sample_payload()["result"].clone(),
            }
        });

        let lines = render_broker_portfolio_groups_mutation_text(&payload);

        assert!(lines.contains(&"action: create".to_string()));
        assert!(lines.contains(&"group_id: group-1".to_string()));
        assert!(lines.contains(&"isins: US0378331005, IE00B4ND3602".to_string()));
        assert!(lines.contains(&"groups: 1".to_string()));
    }
}
