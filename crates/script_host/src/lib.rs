use engine::ScriptSnippet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptError {
    Unsupported { script_count: usize },
}

pub trait ScriptHost {
    fn execute(&mut self, scripts: &[ScriptSnippet]) -> Result<(), ScriptError>;
}

#[derive(Debug, Default)]
pub struct StubScriptHost {
    captured: Vec<ScriptSnippet>,
}

impl StubScriptHost {
    pub fn captured(&self) -> &[ScriptSnippet] {
        &self.captured
    }
}

impl ScriptHost for StubScriptHost {
    fn execute(&mut self, scripts: &[ScriptSnippet]) -> Result<(), ScriptError> {
        self.captured.extend_from_slice(scripts);
        if scripts.is_empty() {
            Ok(())
        } else {
            Err(ScriptError::Unsupported {
                script_count: scripts.len(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_unsupported_for_non_empty_scripts() {
        let mut host = StubScriptHost::default();
        let scripts = vec![ScriptSnippet {
            node_id: 2,
            code: "console.log('hi')".to_string(),
        }];

        let err = host.execute(&scripts).unwrap_err();
        assert_eq!(err, ScriptError::Unsupported { script_count: 1 });
        assert_eq!(host.captured(), scripts.as_slice());
    }

    #[test]
    fn allows_empty_script_list() {
        let mut host = StubScriptHost::default();
        host.execute(&[]).unwrap();
        assert!(host.captured().is_empty());
    }
}
