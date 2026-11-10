use super::Loaded;
use crate::args::Args;

impl Loaded {
    pub fn report(&self, args: &Args) -> serde_json::Value {
        serde_json::json!({
            "version": self.config.version,
            "source": self.path,
            "explicit": self.explicit,
            "loaded": self.present,
            "client": {"prefix": self.prefix(), "escape_ms": self.config.client.escape},
            "host": {"socket": args.socket.as_ref().or(self.runtime.as_ref()), "default": self.runtime.is_some()},
            "history": {"lines": self.config.history.lines, "bytes": self.config.history.bytes, "total": self.config.history.total},
            "workspace": {"name": self.config.workspace.name, "cwd": self.config.workspace.cwd},
            "program": self.config.program.as_ref().map(|program| serde_json::json!({
                "executable": program.executable, "cwd": program.cwd,
                "args": program.args.len(), "environment": program.environment.keys().collect::<Vec<_>>()
            })),
            "interactive_defaults": if self.config.program.is_none() { Some("enter uses invocation SHELL or /bin/sh, invocation cwd and an explicit snapshot of the invocation environment; TERM=xterm-256color") } else { None },
            "bindings": {"prefix": self.config.bindings.prefix, "direct": self.config.bindings.direct},
            "mouse": {"enabled": self.config.mouse.enabled, "resize": self.config.mouse.resize, "copy_on_release": self.config.mouse.release},
            "clipboard": {"target": self.config.clipboard.target, "limit": self.config.clipboard.limit},
            "header": {"enabled": self.config.header.enabled, "components": self.config.header.components, "color": self.config.header.color, "active": self.config.header.active, "inactive": self.config.header.inactive},
            "status": {"components": self.config.status.components, "color": self.config.status.color, "active": self.config.status.active, "inactive": self.config.status.inactive, "separator": self.config.status.separator, "timezone": self.config.status.timezone, "refresh_seconds": self.config.status.refresh},
            "reload": ["client.prefix", "client.escape_ms", "bindings", "mouse", "clipboard", "header", "status"],
            "restart": ["host.socket", "program", "history", "workspace", "locus"],
            "locus": {"configured": self.locus().is_some(), "default": "off", "buffer": "private persistent state/bagua/locus", "failure": "diagnostic only; business result is preserved"},
            "unsupported": [],
        })
    }
}
