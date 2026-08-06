use anyhow::{Result, anyhow};
use serde_json::{Value, json};

use crate::broker_shared::BROKER_PORTFOLIO_GROUP_NOT_FOUND_ERROR_PREFIX;

pub(crate) fn project_broker_portfolio_groups_response(
    response: &Value,
    group_id_filter: Option<&str>,
) -> Result<Value> {
    let inventory = response
        .get("account")
        .and_then(|value| value.get("brokerPortfolio"))
        .and_then(|value| value.get("inventory"))
        .ok_or_else(|| {
            anyhow!("Broker response invalid: missing account.brokerPortfolio.inventory")
        })?;

    let portfolio_groups = inventory
        .get("portfolioGroups")
        .ok_or_else(|| anyhow!("Broker response invalid: missing inventory.portfolioGroups"))?;

    let all_groups = portfolio_groups
        .get("items")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("Broker response invalid: missing portfolioGroups.items"))?;

    let projected_groups = match group_id_filter {
        Some(group_id) => {
            let group = all_groups
                .iter()
                .find(|group| group.get("id").and_then(Value::as_str) == Some(group_id))
                .ok_or_else(|| {
                    anyhow!(
                        "{BROKER_PORTFOLIO_GROUP_NOT_FOUND_ERROR_PREFIX} portfolio group '{group_id}' was not found in the active portfolio"
                    )
                })?;
            vec![map_portfolio_group(group)]
        }
        None => all_groups
            .iter()
            .map(map_portfolio_group)
            .collect::<Vec<_>>(),
    };

    let ungrouped_items = if group_id_filter.is_some() {
        Vec::new()
    } else {
        inventory
            .get("ungroupedInventoryItems")
            .and_then(|value| value.get("items"))
            .and_then(Value::as_array)
            .ok_or_else(|| {
                anyhow!("Broker response invalid: missing inventory.ungroupedInventoryItems.items")
            })?
            .iter()
            .map(map_group_item)
            .collect::<Vec<_>>()
    };

    Ok(json!({
        "offer_allows_additional_group": portfolio_groups
            .get("offerAllowsAdditionalPortfolioGroup")
            .cloned()
            .unwrap_or(Value::Null),
        "max_groups_per_portfolio_reached": portfolio_groups
            .get("maxPortfolioGroupsPerPortfolioReached")
            .cloned()
            .unwrap_or(Value::Null),
        "portfolio_groups": projected_groups,
        "ungrouped_items": ungrouped_items,
    }))
}

fn map_portfolio_group(group: &Value) -> Value {
    let items = group
        .get("items")
        .and_then(Value::as_array)
        .map(|values| values.iter().map(map_group_item).collect::<Vec<_>>())
        .unwrap_or_default();

    json!({
        "group_id": group.get("id").cloned().unwrap_or(Value::Null),
        "name": group
            .get("details")
            .and_then(|value| value.get("name"))
            .cloned()
            .unwrap_or(Value::Null),
        "description": group
            .get("details")
            .and_then(|value| value.get("description"))
            .cloned()
            .unwrap_or(Value::Null),
        "number_of_pending_orders": group
            .get("numberOfPendingOrders")
            .cloned()
            .unwrap_or(Value::Null),
        "savings_plans_amount": group
            .get("savingsPlansAmount")
            .cloned()
            .unwrap_or(Value::Null),
        "performance": map_group_performance(group.get("performance")),
        "items": items,
    })
}

fn map_group_item(item: &Value) -> Value {
    json!({
        "isin": item.get("isin").cloned().unwrap_or(Value::Null),
        "name": item.get("name").cloned().unwrap_or(Value::Null),
        "security_type": item.get("type").cloned().unwrap_or(Value::Null),
    })
}

fn map_group_performance(raw: Option<&Value>) -> Value {
    let Some(performance) = raw.filter(|value| !value.is_null()) else {
        return Value::Null;
    };

    let since_buy = performance
        .get("performancesByTimeframe")
        .and_then(Value::as_array)
        .and_then(|entries| {
            entries
                .iter()
                .find(|entry| entry.get("timeframe").and_then(Value::as_str) == Some("SINCE_BUY"))
        })
        .map(|entry| {
            json!({
                "performance": entry.get("performance").cloned().unwrap_or(Value::Null),
                "simple_absolute_return": entry
                    .get("simpleAbsoluteReturn")
                    .cloned()
                    .unwrap_or(Value::Null),
            })
        })
        .unwrap_or(Value::Null);

    json!({
        "valuation": performance.get("valuation").cloned().unwrap_or(Value::Null),
        "currency": performance.get("currency").cloned().unwrap_or(Value::Null),
        "since_buy": since_buy,
    })
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::project_broker_portfolio_groups_response;
    use crate::machine::classify_error;

    fn sample_response() -> Value {
        json!({
            "account": {
                "brokerPortfolio": {
                    "inventory": {
                        "portfolioGroups": {
                            "offerAllowsAdditionalPortfolioGroup": true,
                            "maxPortfolioGroupsPerPortfolioReached": false,
                            "items": [
                                {
                                    "id": "group-1",
                                    "details": {
                                        "id": "group-1",
                                        "name": "AI portfolio",
                                        "description": "tracked by agent"
                                    },
                                    "numberOfPendingOrders": 1,
                                    "savingsPlansAmount": "25.00",
                                    "performance": {
                                        "id": "performance-1",
                                        "valuation": "1234.56",
                                        "currency": "EUR",
                                        "performancesByTimeframe": [
                                            {
                                                "timeframe": "SINCE_BUY",
                                                "performance": "0.12",
                                                "simpleAbsoluteReturn": "132.45"
                                            },
                                            {
                                                "timeframe": "ONE_DAY",
                                                "performance": "0.01",
                                                "simpleAbsoluteReturn": "10.00"
                                            }
                                        ]
                                    },
                                    "items": [
                                        {
                                            "id": "security-1",
                                            "isin": "US0378331005",
                                            "name": "Apple Inc.",
                                            "type": "STOCK"
                                        }
                                    ]
                                },
                                {
                                    "id": "group-2",
                                    "details": {
                                        "id": "group-2",
                                        "name": "Long term",
                                        "description": null
                                    },
                                    "numberOfPendingOrders": 0,
                                    "savingsPlansAmount": "0",
                                    "performance": null,
                                    "items": []
                                }
                            ]
                        },
                        "ungroupedInventoryItems": {
                            "items": [
                                {
                                    "id": "security-2",
                                    "isin": "IE00B4ND3602",
                                    "name": "MSCI World ETF",
                                    "type": "ETF"
                                }
                            ]
                        }
                    }
                }
            }
        })
    }

    #[test]
    fn project_portfolio_groups_maps_groups_and_since_buy_performance() {
        let projected =
            project_broker_portfolio_groups_response(&sample_response(), None).expect("project");

        assert_eq!(projected["offer_allows_additional_group"], json!(true));
        assert_eq!(projected["max_groups_per_portfolio_reached"], json!(false));
        assert_eq!(
            projected["portfolio_groups"][0],
            json!({
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
            })
        );
        assert_eq!(projected["portfolio_groups"][1]["performance"], Value::Null);
        assert!(
            projected["portfolio_groups"][0]["items"][0]
                .as_object()
                .and_then(|item| item.get("id"))
                .is_none()
        );
        assert_eq!(
            projected["ungrouped_items"][0]["isin"],
            json!("IE00B4ND3602")
        );
        assert!(
            projected["ungrouped_items"][0]
                .as_object()
                .and_then(|item| item.get("id"))
                .is_none()
        );
    }

    #[test]
    fn project_portfolio_groups_filters_to_one_group() {
        let projected =
            project_broker_portfolio_groups_response(&sample_response(), Some("group-2"))
                .expect("project filtered response");

        assert_eq!(
            projected["portfolio_groups"].as_array().map(Vec::len),
            Some(1)
        );
        assert_eq!(
            projected["portfolio_groups"][0]["group_id"],
            json!("group-2")
        );
        assert_eq!(
            projected["ungrouped_items"].as_array().map(Vec::len),
            Some(0)
        );
    }

    #[test]
    fn project_portfolio_groups_filter_miss_uses_stable_not_found_error_contract() {
        let err =
            project_broker_portfolio_groups_response(&sample_response(), Some("missing-group"))
                .expect_err("missing group should fail");

        assert_eq!(classify_error(&err).code, "portfolio_group_not_found");
        assert!(
            err.to_string()
                .contains("portfolio group 'missing-group' was not found in the active portfolio")
        );
    }
}
