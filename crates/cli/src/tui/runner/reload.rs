use super::Runner;

impl Runner {
    pub(crate) fn configure(mut self, config: crate::config::Loaded) -> Self {
        self.config = config;
        self.delay();
        self
    }

    fn delay(&mut self) {
        self.keyboard.delay(self.config.delay());
        self.app
            .preferences(self.config.mouse(), self.config.clipboard());
        self.app.decorate(self.config.decor().clone());
        self.header = None;
        self.size = (0, 0);
    }

    pub(super) fn reload(&mut self) {
        match self.config.reload() {
            Ok(()) => {
                self.delay();
                self.app
                    .status("Configuration reloaded; host and programs preserved");
            }
            Err(error) => self
                .app
                .error(format!("Configuration unchanged: {}", error.message())),
        }
    }
}
