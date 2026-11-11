use crate::{
    App,
    template::{Context, Field},
};
use workspace::{Id, Window};

pub(super) struct Frame<'a> {
    pub app: &'a App,
    pub pane: Option<Id>,
    pub window: Option<Id>,
    pub width: usize,
}

impl<'a> Frame<'a> {
    pub fn new(app: &'a App, pane: Option<Id>) -> Self {
        Self {
            app,
            pane,
            window: None,
            width: usize::from(app.viewport.width()),
        }
    }

    fn window(&self) -> Option<&Window> {
        let session = self.app.session()?;
        session.windows().find(|w| match (self.window, self.pane) {
            (Some(id), _) => w.id() == id,
            (_, Some(id)) => w.panes().any(|p| p.id() == id),
            _ => Some(w.id()) == session.selected(),
        })
    }

    fn pane(&self) -> Option<Id> {
        self.pane.or_else(|| self.window().map(Window::focus))
    }

    fn title(&self) -> String {
        let Some(window) = self.window() else {
            return String::new();
        };
        if window.automatic()
            && let Some(context) = self.app.observed(window.focus())
        {
            return context.command.clone();
        }
        window.name().into()
    }

    fn metadata(&self, field: Field) -> String {
        let Some(context) = self.pane().and_then(|pane| self.app.observed(pane)) else {
            return String::new();
        };
        match field {
            Field::Command => context.command.clone(),
            Field::Directory => context.cwd.clone(),
            Field::Leaf => context
                .cwd
                .trim_end_matches('/')
                .rsplit('/')
                .next()
                .filter(|s| !s.is_empty())
                .unwrap_or("/")
                .into(),
            _ => String::new(),
        }
    }
}

impl Context for Frame<'_> {
    fn field(&self, field: Field) -> String {
        match field {
            Field::Session => self.app.session().map_or("", |s| s.name()).into(),
            Field::Window => self
                .app
                .session()
                .and_then(|s| {
                    s.windows()
                        .position(|w| Some(w.id()) == self.window().map(Window::id))
                })
                .map_or(String::new(), |i| (i + 1).to_string()),
            Field::Name => self.window().map_or("", Window::name).into(),
            Field::Title => self.title(),
            Field::Selected => flag(
                self.window()
                    .is_some_and(|w| self.app.session().and_then(|s| s.selected()) == Some(w.id())),
            ),
            Field::Pane => self
                .window()
                .and_then(|w| w.panes().position(|p| Some(p.id()) == self.pane()))
                .map_or(String::new(), |i| (i + 1).to_string()),
            Field::Identity => self.pane().map_or(String::new(), |p| p.value().to_string()),
            Field::Command | Field::Directory | Field::Leaf => self.metadata(field),
            Field::Active => flag(self.pane().is_some_and(|p| self.app.focus() == Some(p))),
            Field::State => self.pane().map_or("?", |p| self.app.activity(p)).into(),
            Field::Role => if self.app.controller() {
                "control"
            } else {
                "observe"
            }
            .into(),
            Field::Mode => self.app.mode(),
            Field::Columns => self.app.viewport.width().to_string(),
            Field::Copy => flag(self.app.mode().contains("Copy")),
            Field::Recovery => flag(self.app.recovering),
            Field::Observer => flag(!self.app.controller()),
            Field::Prefix => self.app.leader.clone().unwrap_or_default(),
            Field::Waiting => flag(self.app.leader.is_some()),
            Field::Notice => self.app.notice.clone(),
            Field::Clock => self.app.clock.clone(),
            Field::Date => self.app.date.clone(),
        }
    }

    fn windows(&self) -> Vec<Self> {
        self.app
            .session()
            .into_iter()
            .flat_map(|s| s.windows())
            .map(|w| Self {
                app: self.app,
                pane: None,
                window: Some(w.id()),
                width: self.width,
            })
            .collect()
    }

    fn width(&self) -> usize {
        self.width
    }
}

fn flag(value: bool) -> String {
    if value { "1" } else { "" }.into()
}
