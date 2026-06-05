use std::path::PathBuf;

use anyhow::Result;

use crate::config::{ConnectionConfig, ConnectionStore, Language};
use crate::i18n::Messages;

pub struct App {
    pub config_path: PathBuf,
    pub store: ConnectionStore,
    pub selected: usize,
    pub messages: Messages,
    pub status: Option<String>,
}

impl App {
    pub fn new(config_path: PathBuf, store: ConnectionStore, messages: Messages) -> Self {
        Self {
            config_path,
            store,
            selected: 0,
            messages,
            status: None,
        }
    }

    pub fn selected_connection(&self) -> Option<&ConnectionConfig> {
        self.store.connections.get(self.selected)
    }

    pub fn add_connection(&mut self, connection: ConnectionConfig) -> Result<()> {
        self.store.connections.push(connection);
        self.selected = self.store.connections.len().saturating_sub(1);
        self.save()
    }

    pub fn delete_selected(&mut self) -> Result<()> {
        if self.store.connections.is_empty() {
            self.status = Some(self.messages.no_connection_to_delete.to_owned());
            return Ok(());
        }

        self.store.connections.remove(self.selected);
        self.selected = self
            .selected
            .min(self.store.connections.len().saturating_sub(1));
        self.save()?;
        self.status = Some(self.messages.connection_deleted.to_owned());
        Ok(())
    }

    pub fn save(&self) -> Result<()> {
        self.store.save(&self.config_path)
    }

    pub fn change_language(&mut self, language: Language) -> Result<()> {
        self.store.language = language;
        self.messages = Messages::for_language(language);
        self.status = Some(self.messages.language_saved.to_owned());
        self.save()
    }
}
