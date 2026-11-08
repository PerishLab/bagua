use super::Runner;

impl Runner {
    pub(crate) fn configure(mut self, config: crate::config::Loaded) -> Self {
        self.config = config;
        self.delay();
        self
    }

    fn delay(&mut self) {
        self.keyboard.delay(self.config.delay());
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
                .status(format!("Configuration unchanged: {}", error.message())),
        }
    }
}
