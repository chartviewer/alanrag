use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryMetrics {
    pub query: String,
    pub top_score: f32,
    pub result_count: usize,
    pub response_time_ms: u64,
    pub timestamp: DateTime<Utc>,
    pub search_method: String, // "semantic", "keyword", "hybrid"
    pub intent: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceStats {
    pub total_queries: usize,
    pub avg_response_time_ms: f64,
    pub avg_relevance_score: f64,
    pub queries_by_intent: HashMap<String, usize>,
    pub search_method_usage: HashMap<String, usize>,
    pub score_distribution: ScoreDistribution,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreDistribution {
    pub excellent: usize,    // > 0.8
    pub good: usize,         // 0.6 - 0.8
    pub fair: usize,         // 0.4 - 0.6
    pub poor: usize,         // < 0.4
}

/// Performance metrics collector for RAG system
pub struct PerformanceMetrics {
    queries: Arc<Mutex<Vec<QueryMetrics>>>,
    start_time: Instant,
}

impl PerformanceMetrics {
    pub fn new() -> Self {
        Self {
            queries: Arc::new(Mutex::new(Vec::new())),
            start_time: Instant::now(),
        }
    }

    /// Record a search query and its results
    pub fn record_query(
        &self,
        query: &str,
        top_score: f32,
        result_count: usize,
        response_time: Duration,
        search_method: &str,
        intent: &str,
    ) {
        let metric = QueryMetrics {
            query: query.to_string(),
            top_score,
            result_count,
            response_time_ms: response_time.as_millis() as u64,
            timestamp: Utc::now(),
            search_method: search_method.to_string(),
            intent: intent.to_string(),
        };

        if let Ok(mut queries) = self.queries.lock() {
            queries.push(metric);

            // Log for monitoring
            eprintln!(
                "🔍 Query: '{}' | Score: {:.3} | Time: {}ms | Method: {} | Intent: {}",
                query, top_score, response_time.as_millis(), search_method, intent
            );

            // Alert on poor performance
            if top_score < 0.4 {
                eprintln!("⚠️  LOW RELEVANCE: Query '{}' scored {:.3}", query, top_score);
            }

            if response_time.as_millis() > 200 {
                eprintln!("⚠️  SLOW RESPONSE: Query '{}' took {}ms", query, response_time.as_millis());
            }
        }
    }

    /// Get comprehensive performance statistics
    pub fn get_stats(&self) -> PerformanceStats {
        if let Ok(queries) = self.queries.lock() {
            if queries.is_empty() {
                return PerformanceStats {
                    total_queries: 0,
                    avg_response_time_ms: 0.0,
                    avg_relevance_score: 0.0,
                    queries_by_intent: HashMap::new(),
                    search_method_usage: HashMap::new(),
                    score_distribution: ScoreDistribution {
                        excellent: 0,
                        good: 0,
                        fair: 0,
                        poor: 0,
                    },
                };
            }

            let total_queries = queries.len();

            // Calculate averages
            let total_time: u64 = queries.iter().map(|q| q.response_time_ms).sum();
            let avg_response_time_ms = total_time as f64 / total_queries as f64;

            let total_score: f32 = queries.iter().map(|q| q.top_score).sum();
            let avg_relevance_score = total_score as f64 / total_queries as f64;

            // Count by intent
            let mut queries_by_intent = HashMap::new();
            for query in queries.iter() {
                *queries_by_intent.entry(query.intent.clone()).or_insert(0) += 1;
            }

            // Count by search method
            let mut search_method_usage = HashMap::new();
            for query in queries.iter() {
                *search_method_usage.entry(query.search_method.clone()).or_insert(0) += 1;
            }

            // Score distribution
            let mut score_distribution = ScoreDistribution {
                excellent: 0,
                good: 0,
                fair: 0,
                poor: 0,
            };

            for query in queries.iter() {
                match query.top_score {
                    score if score > 0.8 => score_distribution.excellent += 1,
                    score if score > 0.6 => score_distribution.good += 1,
                    score if score > 0.4 => score_distribution.fair += 1,
                    _ => score_distribution.poor += 1,
                }
            }

            PerformanceStats {
                total_queries,
                avg_response_time_ms,
                avg_relevance_score,
                queries_by_intent,
                search_method_usage,
                score_distribution,
            }
        } else {
            PerformanceStats {
                total_queries: 0,
                avg_response_time_ms: 0.0,
                avg_relevance_score: 0.0,
                queries_by_intent: HashMap::new(),
                search_method_usage: HashMap::new(),
                score_distribution: ScoreDistribution {
                    excellent: 0,
                    good: 0,
                    fair: 0,
                    poor: 0,
                },
            }
        }
    }

    /// Get recent performance trends
    pub fn get_recent_performance(&self, minutes: u64) -> PerformanceStats {
        let cutoff = Utc::now() - chrono::Duration::minutes(minutes as i64);

        if let Ok(queries) = self.queries.lock() {
            let recent_queries: Vec<&QueryMetrics> = queries
                .iter()
                .filter(|q| q.timestamp > cutoff)
                .collect();

            if recent_queries.is_empty() {
                return PerformanceStats {
                    total_queries: 0,
                    avg_response_time_ms: 0.0,
                    avg_relevance_score: 0.0,
                    queries_by_intent: HashMap::new(),
                    search_method_usage: HashMap::new(),
                    score_distribution: ScoreDistribution {
                        excellent: 0,
                        good: 0,
                        fair: 0,
                        poor: 0,
                    },
                };
            }

            // Similar calculations but for recent queries only
            let total_queries = recent_queries.len();
            let avg_response_time_ms = recent_queries.iter().map(|q| q.response_time_ms).sum::<u64>() as f64 / total_queries as f64;
            let avg_relevance_score = recent_queries.iter().map(|q| q.top_score).sum::<f32>() as f64 / total_queries as f64;

            let mut queries_by_intent = HashMap::new();
            let mut search_method_usage = HashMap::new();
            let mut score_distribution = ScoreDistribution {
                excellent: 0,
                good: 0,
                fair: 0,
                poor: 0,
            };

            for query in &recent_queries {
                *queries_by_intent.entry(query.intent.clone()).or_insert(0) += 1;
                *search_method_usage.entry(query.search_method.clone()).or_insert(0) += 1;

                match query.top_score {
                    score if score > 0.8 => score_distribution.excellent += 1,
                    score if score > 0.6 => score_distribution.good += 1,
                    score if score > 0.4 => score_distribution.fair += 1,
                    _ => score_distribution.poor += 1,
                }
            }

            PerformanceStats {
                total_queries,
                avg_response_time_ms,
                avg_relevance_score,
                queries_by_intent,
                search_method_usage,
                score_distribution,
            }
        } else {
            PerformanceStats {
                total_queries: 0,
                avg_response_time_ms: 0.0,
                avg_relevance_score: 0.0,
                queries_by_intent: HashMap::new(),
                search_method_usage: HashMap::new(),
                score_distribution: ScoreDistribution {
                    excellent: 0,
                    good: 0,
                    fair: 0,
                    poor: 0,
                },
            }
        }
    }

    /// Export metrics in a structured format
    pub fn export_metrics(&self) -> String {
        let stats = self.get_stats();
        let uptime = self.start_time.elapsed();

        format!(
            r#"
📊 RAG Performance Metrics
==========================

🔍 Query Statistics:
   Total Queries: {}
   Average Response Time: {:.1}ms
   Average Relevance Score: {:.3}
   Uptime: {:.1} hours

📈 Score Distribution:
   Excellent (>0.8): {} ({:.1}%)
   Good (0.6-0.8):   {} ({:.1}%)
   Fair (0.4-0.6):   {} ({:.1}%)
   Poor (<0.4):      {} ({:.1}%)

🎯 Query Intents:
{}

🔧 Search Methods:
{}

🎯 Performance Targets:
   Target Avg Score: >0.80 (Current: {:.3})
   Target Response Time: <150ms (Current: {:.1}ms)
   Target Excellent Rate: >80% (Current: {:.1}%)
"#,
            stats.total_queries,
            stats.avg_response_time_ms,
            stats.avg_relevance_score,
            uptime.as_secs_f64() / 3600.0,
            stats.score_distribution.excellent,
            (stats.score_distribution.excellent as f64 / stats.total_queries.max(1) as f64) * 100.0,
            stats.score_distribution.good,
            (stats.score_distribution.good as f64 / stats.total_queries.max(1) as f64) * 100.0,
            stats.score_distribution.fair,
            (stats.score_distribution.fair as f64 / stats.total_queries.max(1) as f64) * 100.0,
            stats.score_distribution.poor,
            (stats.score_distribution.poor as f64 / stats.total_queries.max(1) as f64) * 100.0,
            stats.queries_by_intent
                .iter()
                .map(|(intent, count)| format!("   {}: {}", intent, count))
                .collect::<Vec<_>>()
                .join("\n"),
            stats.search_method_usage
                .iter()
                .map(|(method, count)| format!("   {}: {}", method, count))
                .collect::<Vec<_>>()
                .join("\n"),
            stats.avg_relevance_score,
            stats.avg_response_time_ms,
            (stats.score_distribution.excellent as f64 / stats.total_queries.max(1) as f64) * 100.0,
        )
    }

    /// Get queries that performed poorly for analysis
    pub fn get_poor_queries(&self, min_score: f32) -> Vec<QueryMetrics> {
        if let Ok(queries) = self.queries.lock() {
            queries
                .iter()
                .filter(|q| q.top_score < min_score)
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Clear old metrics (keep last N)
    pub fn cleanup_old_metrics(&self, keep_last: usize) {
        if let Ok(mut queries) = self.queries.lock() {
            let queries_len = queries.len();
            if queries_len > keep_last {
                queries.drain(0..queries_len - keep_last);
            }
        }
    }
}

/// Simple timer for measuring operation duration
pub struct Timer {
    start: Instant,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_metrics_collection() {
        let metrics = PerformanceMetrics::new();

        metrics.record_query(
            "test query",
            0.85,
            5,
            Duration::from_millis(100),
            "hybrid",
            "concept"
        );

        let stats = metrics.get_stats();
        assert_eq!(stats.total_queries, 1);
        assert_eq!(stats.score_distribution.excellent, 1);
        assert_eq!(stats.avg_response_time_ms, 100.0);
    }

    #[test]
    fn test_timer() {
        let timer = Timer::new();
        thread::sleep(Duration::from_millis(10));
        let elapsed = timer.elapsed();
        assert!(elapsed.as_millis() >= 10);
    }
}