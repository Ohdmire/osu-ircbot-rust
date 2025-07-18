
use std::collections::HashSet;
use regex::Regex;
use rusqlite::Connection;
use serde::{Serialize, Deserialize};
use std::path::Path;

#[derive(Debug)]
#[derive(Clone)]
pub struct ChartQuery {
    pub competition: Option<String>,
    pub season: Option<String>,
    pub pool_index: Option<i32>,
    pub chart_type: Option<String>,
    pub chart_type_index: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Chart {
    pub competition_name: Option<String>,
    pub season: Option<String>,
    pub pool_name: Option<String>,
    pub pool_index: Option<i32>,
    pub chart_type: Option<String>,
    pub chart_type_index: Option<i32>,
    pub chart_id: i32,
}



impl ChartQuery {
    pub fn parse(input: &str) -> Result<Self, String> {
        let mod_types: HashSet<&str> = ["NM", "HD", "HR", "DT", "FM","TB"].iter().cloned().collect();
        let input = input.trim().trim_start_matches("!pick").trim();

        // 改进后的正则表达式，明确处理连字符格式
        let re = Regex::new(r"(?i)^(?:(?P<comp>[^\s-]+)(?:\s+|$))?(?:(?P<season>S\d+)(?:-(?P<pool>\d+))?(?:\s+|$))?(?:(?P<mod>NM|HD|HR|DT|FM|TB)(?P<mod_idx>\d*)(?:\s*|$))?$").unwrap();

        let caps = re.captures(input).ok_or("Invalid query format")?;

        // 提取各个部分
        let mut competition = caps.name("comp").map(|m| m.as_str().to_string());
        let season = caps.name("season").map(|m| m.as_str().to_string());
        let pool_index = caps.name("pool").and_then(|m| m.as_str().parse().ok());
        let mut chart_type = caps.name("mod").map(|m| m.as_str().to_string());
        let mut chart_type_index = caps.name("mod_idx").and_then(|m| {
            let s = m.as_str();
            if s.is_empty() { None } else { s.parse().ok() }
        });

        // 处理只有"NM1"这种形式的情况
        if competition.is_some() && chart_type.is_none() {
            if let Some(ref comp) = competition {
                // 检查是否以mod类型开头
                for mod_len in (2..=comp.len()).rev() {
                    let prefix = &comp[..mod_len.min(comp.len())];
                    if mod_types.contains(prefix.to_uppercase().as_str()) {
                        // 分离mod类型和序号
                        chart_type = Some(prefix.to_uppercase());
                        let remainder = &comp[mod_len..];
                        if !remainder.is_empty() {
                            chart_type_index = remainder.parse().ok();
                        }
                        competition = None;
                        break;
                    }
                }
            }
        }

        Ok(Self {
            competition,
            season,
            pool_index,
            chart_type,
            chart_type_index,
        })
    }

    pub fn to_sql(&self) -> (String, Vec<rusqlite::types::Value>) {
        let mut conditions = Vec::new();
        let mut params = Vec::new();

        if let Some(ref comp) = self.competition {
            conditions.push("competition_name = ?");
            params.push(comp.clone().into());
        }

        if let Some(ref season) = self.season {
            conditions.push("season = ?");
            params.push(season.clone().into());
        }

        if let Some(pool_idx) = self.pool_index {
            conditions.push("pool_index = ?");
            params.push(pool_idx.into());
        }

        if let Some(ref chart_type) = self.chart_type {
            conditions.push("chart_type = ?");
            params.push(chart_type.clone().into());

            if let Some(type_idx) = self.chart_type_index {
                conditions.push("chart_type_index = ?");
                params.push(type_idx.into());
            }
        }

        let where_clause = if !conditions.is_empty() {
            format!("WHERE {}", conditions.join(" AND "))
        } else {
            String::new()
        };

        let sql = format!("SELECT * FROM charts {} ORDER BY RANDOM() LIMIT 1", where_clause);

        (sql, params)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_queries() {
        let test_cases = vec![
            ("!pick HD", (
                None, None, None, Some("HD"), None
            )),
            ("!pick HD1", (
                None, None, None, Some("HD"), Some(1)
            )),
            ("!pick MP5", (
                Some("MP5"), None, None, None, None
            )),
            ("!pick MP5 HD", (
                Some("MP5"), None, None, Some("HD"), None
            )),
            ("!pick MP5 HD1", (
                Some("MP5"), None, None, Some("HD"), Some(1)
            )),
            ("!pick MP5 S22", (
                Some("MP5"), Some("S22"), None, None, None
            )),
            ("!pick MP5 S22 HD", (
                Some("MP5"), Some("S22"), None, Some("HD"), None
            )),
            ("!pick MP5 S22 HD1", (
                Some("MP5"), Some("S22"), None, Some("HD"), Some(1)
            )),
            ("!pick MP5 S22-1", (
                Some("MP5"), Some("S22"), Some(1), None, None
            )),
            ("!pick MP5 S22-1 HD", (
                Some("MP5"), Some("S22"), Some(1), Some("HD"), None
            )),
            ("!pick MP5 S22-1 HD1", (
                Some("MP5"), Some("S22"), Some(1), Some("HD"), Some(1)
            )),
        ];

        for (input, expected) in test_cases {
            let result = ChartQuery::parse(input).unwrap();
            assert_eq!(result.competition.as_deref(), expected.0, "Failed on competition for: {}", input);
            assert_eq!(result.season.as_deref(), expected.1, "Failed on season for: {}", input);
            assert_eq!(result.pool_index, expected.2, "Failed on pool_index for: {}", input);
            assert_eq!(result.chart_type.as_deref(), expected.3, "Failed on chart_type for: {}", input);
            assert_eq!(result.chart_type_index, expected.4, "Failed on chart_type_index for: {}", input);
        }
    }
}

pub struct ChartDatabase {
    conn: Connection,
}

impl ChartDatabase {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(path)?;
        Ok(Self { conn })
    }

    pub fn query_random_chart(&self, query: &ChartQuery) -> Result<Option<Chart>, rusqlite::Error> {
        let (sql, params) = query.to_sql();

        // 准备查询参数
        let mut stmt = self.conn.prepare(&sql)?;

        // 将参数转换为rusqlite需要的格式
        let mut params_vec = Vec::new();
        for param in params {
            params_vec.push(param);
        }

        // 执行查询
        let mut rows = stmt.query_map(rusqlite::params_from_iter(params_vec), |row| {
            Ok(Chart {
                competition_name: row.get(0)?,
                season: row.get(1)?,
                pool_name: row.get(2)?,
                pool_index: row.get(3)?,
                chart_type: row.get(4)?,
                chart_type_index: row.get(5)?,
                chart_id: row.get(6)?,
            })
        })?;

        // 获取第一个结果
        if let Some(row) = rows.next() {
            row.map(Some)
        } else {
            Ok(None)
        }
    }

    pub fn query_with_fallback(&self, query: &ChartQuery) -> Result<Option<Chart>, rusqlite::Error> {
        // 尝试原始查询
        if let Some(chart) = self.query_random_chart(query)? {
            return Ok(Some(chart));
        }

        // 第一次回退：移除pool_index条件
        if query.pool_index.is_some() {
            let mut fallback1 = query.clone();
            fallback1.pool_index = None;
            if let Some(chart) = self.query_random_chart(&fallback1)? {
                return Ok(Some(chart));
            }
        }

        // 第二次回退：移除chart_type_index条件
        if query.chart_type_index.is_some() {
            let mut fallback2 = query.clone();
            fallback2.chart_type_index = None;
            if let Some(chart) = self.query_random_chart(&fallback2)? {
                return Ok(Some(chart));
            }
        }

        // 第三次回退：移除season条件
        if query.season.is_some() {
            let mut fallback3 = query.clone();
            fallback3.season = None;
            if let Some(chart) = self.query_random_chart(&fallback3)? {
                return Ok(Some(chart));
            }
        }

        // 所有回退尝试后仍无结果
        Ok(None)
    }

}


// fn main() -> Result<(), Box<dyn std::error::Error>> {
//     // 初始化数据库连接
//     let db = ChartDatabase::open("charts.sqlite")?;
//
//     // 解析查询
//     let query = ChartQuery::parse("!pick MP5 S22-1 NM1")?;
//     println!("解析后的查询条件: {:?}", query);
//
//     // 执行查询
//     if let Some(chart) = db.query_with_fallback(&query)? {
//         println!("查询结果: {}", serde_json::to_string_pretty(&chart)?);
//
//         // 使用结果示例
//         println!("找到谱面: ID={}, 类型={}{}",
//                  chart.chart_id,
//                  chart.chart_type.unwrap_or_default(),
//                  chart.chart_type_index.map(|i| i.to_string()).unwrap_or_default());
//     } else {
//         println!("没有找到匹配的谱面");
//     }
//
//     Ok(())
// }
