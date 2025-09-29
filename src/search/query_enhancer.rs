use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum QueryIntent {
    CodeExample,    // Looking for code implementation
    Concept,        // Looking for explanation/definition
    Configuration,  // Looking for config/setup
    Mixed,          // Multiple intents
}

#[derive(Debug, Clone)]
pub struct EnhancedQuery {
    pub original: String,
    pub enhanced: String,
    pub intent: QueryIntent,
    pub keywords: Vec<String>,
    pub uvm_terms: Vec<String>,
}

/// Query enhancer specifically designed for UVM/SystemVerilog content
pub struct QueryEnhancer {
    uvm_synonyms: HashMap<String, Vec<String>>,
    abbreviations: HashMap<String, String>,
    code_indicators: Vec<String>,
    concept_indicators: Vec<String>,
}

impl QueryEnhancer {
    pub fn new() -> Self {
        let mut uvm_synonyms = HashMap::new();

        // UVM class and component synonyms
        uvm_synonyms.insert("monitor".to_string(), vec![
            "uvm_monitor".to_string(),
            "monitoring".to_string(),
            "observer".to_string(),
            "watcher".to_string(),
        ]);

        uvm_synonyms.insert("driver".to_string(), vec![
            "uvm_driver".to_string(),
            "stimulus".to_string(),
            "generator".to_string(),
        ]);

        uvm_synonyms.insert("agent".to_string(), vec![
            "uvm_agent".to_string(),
            "verification_component".to_string(),
            "vc".to_string(),
        ]);

        uvm_synonyms.insert("scoreboard".to_string(), vec![
            "uvm_scoreboard".to_string(),
            "checker".to_string(),
            "comparator".to_string(),
            "validator".to_string(),
        ]);

        // Configuration and database terms
        uvm_synonyms.insert("config_db".to_string(), vec![
            "uvm_config_db".to_string(),
            "configuration_database".to_string(),
            "config".to_string(),
            "configuration".to_string(),
        ]);

        uvm_synonyms.insert("resource_db".to_string(), vec![
            "uvm_resource_db".to_string(),
            "resource_database".to_string(),
        ]);

        // Phase and lifecycle terms
        uvm_synonyms.insert("phase".to_string(), vec![
            "uvm_phase".to_string(),
            "build_phase".to_string(),
            "connect_phase".to_string(),
            "run_phase".to_string(),
            "extract_phase".to_string(),
            "check_phase".to_string(),
            "report_phase".to_string(),
        ]);

        // TLM and communication
        uvm_synonyms.insert("tlm".to_string(), vec![
            "transaction_level_modeling".to_string(),
            "analysis_port".to_string(),
            "analysis_export".to_string(),
            "analysis_imp".to_string(),
            "tlm_analysis_port".to_string(),
        ]);

        // Factory and utilities
        uvm_synonyms.insert("factory".to_string(), vec![
            "uvm_factory".to_string(),
            "component_utils".to_string(),
            "object_utils".to_string(),
            "type_override".to_string(),
        ]);

        // Test and sequence related
        uvm_synonyms.insert("sequence".to_string(), vec![
            "uvm_sequence".to_string(),
            "uvm_sequence_item".to_string(),
            "sequencer".to_string(),
            "uvm_sequencer".to_string(),
        ]);

        let mut abbreviations = HashMap::new();
        abbreviations.insert("cfg".to_string(), "configuration".to_string());
        abbreviations.insert("db".to_string(), "database".to_string());
        abbreviations.insert("tlm".to_string(), "transaction_level_modeling".to_string());
        abbreviations.insert("vc".to_string(), "verification_component".to_string());
        abbreviations.insert("tb".to_string(), "testbench".to_string());
        abbreviations.insert("env".to_string(), "environment".to_string());

        let code_indicators = vec![
            "implementation".to_string(),
            "code".to_string(),
            "example".to_string(),
            "sample".to_string(),
            "snippet".to_string(),
            "function".to_string(),
            "method".to_string(),
            "class".to_string(),
            "task".to_string(),
        ];

        let concept_indicators = vec![
            "what".to_string(),
            "define".to_string(),
            "explain".to_string(),
            "description".to_string(),
            "overview".to_string(),
            "purpose".to_string(),
            "meaning".to_string(),
            "concept".to_string(),
        ];

        Self {
            uvm_synonyms,
            abbreviations,
            code_indicators,
            concept_indicators,
        }
    }

    pub fn enhance(&self, query: &str) -> EnhancedQuery {
        let original = query.to_string();
        let normalized = query.to_lowercase();

        // 1. Expand abbreviations
        let expanded = self.expand_abbreviations(&normalized);

        // 2. Detect intent
        let intent = self.detect_intent(&expanded);

        // 3. Extract UVM-specific terms
        let uvm_terms = self.extract_uvm_terms(&expanded);

        // 4. Add synonyms for better matching
        let with_synonyms = self.add_synonyms(&expanded);

        // 5. Build enhanced query based on intent
        let enhanced = self.build_enhanced_query(&with_synonyms, &intent, &uvm_terms);

        // 6. Extract important keywords
        let keywords = self.extract_keywords(&enhanced);

        EnhancedQuery {
            original,
            enhanced,
            intent,
            keywords,
            uvm_terms,
        }
    }

    fn expand_abbreviations(&self, query: &str) -> String {
        let mut result = query.to_string();

        for (abbrev, expansion) in &self.abbreviations {
            // Match abbreviation as whole word
            let pattern = format!(r"\b{}\b", regex::escape(abbrev));
            if let Ok(re) = regex::Regex::new(&pattern) {
                result = re.replace_all(&result, expansion).to_string();
            }
        }

        result
    }

    fn detect_intent(&self, query: &str) -> QueryIntent {
        let has_code_intent = self.code_indicators.iter()
            .any(|indicator| query.contains(indicator));

        let has_concept_intent = self.concept_indicators.iter()
            .any(|indicator| query.contains(indicator));

        let has_config_intent = query.contains("config") ||
                               query.contains("setup") ||
                               query.contains("configuration");

        match (has_code_intent, has_concept_intent, has_config_intent) {
            (true, false, false) => QueryIntent::CodeExample,
            (false, true, false) => QueryIntent::Concept,
            (false, false, true) => QueryIntent::Configuration,
            _ => QueryIntent::Mixed,
        }
    }

    fn extract_uvm_terms(&self, query: &str) -> Vec<String> {
        let mut uvm_terms = Vec::new();

        // Look for UVM-specific patterns
        let uvm_patterns = vec![
            r"uvm_\w+",           // uvm_config_db, uvm_object, etc.
            r"`uvm_\w+",          // `uvm_component_utils, etc.
            r"\w+_phase",         // build_phase, run_phase, etc.
            r"\w+_imp\b",         // analysis_imp, etc.
            r"\w+_export\b",      // analysis_export, etc.
            r"\w+_port\b",        // analysis_port, etc.
        ];

        for pattern in &uvm_patterns {
            if let Ok(regex) = regex::Regex::new(pattern) {
                for mat in regex.find_iter(query) {
                    uvm_terms.push(mat.as_str().to_string());
                }
            }
        }

        uvm_terms
    }

    fn add_synonyms(&self, query: &str) -> String {
        let mut enhanced = query.to_string();

        for (term, synonyms) in &self.uvm_synonyms {
            if query.contains(term) {
                // Add the most relevant synonym
                if let Some(primary_synonym) = synonyms.first() {
                    enhanced.push_str(&format!(" {}", primary_synonym));
                }
            }
        }

        enhanced
    }

    fn build_enhanced_query(&self, query: &str, intent: &QueryIntent, uvm_terms: &[String]) -> String {
        let mut enhanced = query.to_string();

        // Add intent-specific context
        match intent {
            QueryIntent::CodeExample => {
                if !enhanced.contains("implementation") && !enhanced.contains("example") {
                    enhanced.push_str(" implementation example code");
                }
            }
            QueryIntent::Concept => {
                if !enhanced.contains("definition") && !enhanced.contains("explanation") {
                    enhanced.push_str(" definition explanation");
                }
            }
            QueryIntent::Configuration => {
                if !enhanced.contains("setup") && !enhanced.contains("configuration") {
                    enhanced.push_str(" setup configuration");
                }
            }
            QueryIntent::Mixed => {
                // Don't add specific context for mixed intent
            }
        }

        // Add UVM context if UVM terms are detected
        if !uvm_terms.is_empty() && !enhanced.contains("uvm") {
            enhanced.push_str(" uvm verification");
        }

        enhanced
    }

    fn extract_keywords(&self, query: &str) -> Vec<String> {
        query.split_whitespace()
            .map(|word| {
                word.chars()
                    .filter(|c| c.is_alphanumeric() || *c == '_')
                    .collect::<String>()
            })
            .filter(|word| !word.is_empty() && word.len() > 2)
            .collect()
    }

    /// Get UVM-specific boost terms for a query
    pub fn get_boost_terms(&self, query: &str) -> Vec<(String, f32)> {
        let mut boost_terms = Vec::new();

        // If query contains UVM terms, boost related UVM patterns
        if query.contains("uvm") || query.contains("verification") {
            boost_terms.push(("uvm_".to_string(), 1.5));
            boost_terms.push(("verification".to_string(), 1.3));
            boost_terms.push(("testbench".to_string(), 1.2));
        }

        if query.contains("config") {
            boost_terms.push(("uvm_config_db".to_string(), 2.0));
            boost_terms.push(("configuration".to_string(), 1.4));
        }

        if query.contains("phase") {
            boost_terms.push(("build_phase".to_string(), 1.5));
            boost_terms.push(("connect_phase".to_string(), 1.5));
            boost_terms.push(("run_phase".to_string(), 1.5));
        }

        boost_terms
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uvm_query_enhancement() {
        let enhancer = QueryEnhancer::new();

        let result = enhancer.enhance("what is uvm config db");
        assert!(result.enhanced.contains("uvm_config_db"));
        assert!(result.enhanced.contains("configuration_database"));
        assert!(matches!(result.intent, QueryIntent::Concept));
        assert!(!result.uvm_terms.is_empty());
    }

    #[test]
    fn test_code_intent_detection() {
        let enhancer = QueryEnhancer::new();

        let result = enhancer.enhance("show me uvm monitor implementation");
        assert!(matches!(result.intent, QueryIntent::CodeExample));
        assert!(result.enhanced.contains("implementation"));
    }

    #[test]
    fn test_abbreviation_expansion() {
        let enhancer = QueryEnhancer::new();

        let result = enhancer.enhance("cfg db setup");
        assert!(result.enhanced.contains("configuration"));
        assert!(result.enhanced.contains("database"));
    }

    #[test]
    fn test_uvm_term_extraction() {
        let enhancer = QueryEnhancer::new();

        let result = enhancer.enhance("uvm_config_db and build_phase example");
        assert!(result.uvm_terms.contains(&"uvm_config_db".to_string()));
        assert!(result.uvm_terms.contains(&"build_phase".to_string()));
    }
}