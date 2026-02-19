#[derive(Clone, Debug)]
pub struct EnvFilter(String);

impl EnvFilter {
    pub fn try_from_default_env() -> Result<Self, ()> {
        std::env::var("RUST_LOG").map(Self).map_err(|_| ())
    }

    pub fn new(filter: &str) -> Self {
        Self(filter.to_string())
    }
}

pub struct FmtBuilder {
    filter: EnvFilter,
}

impl FmtBuilder {
    pub fn with_env_filter(mut self, filter: EnvFilter) -> Self {
        self.filter = filter;
        self
    }

    pub fn init(self) {
        let _ = self.filter.0;
    }
}

pub fn fmt() -> FmtBuilder {
    FmtBuilder {
        filter: EnvFilter::new("info"),
    }
}
