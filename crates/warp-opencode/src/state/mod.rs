use crate::api::events::OpenCodeEvent;
use crate::api::schema::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting { attempt: u32 },
    Error(String),
}

#[derive(Debug, Clone, Default)]
pub struct SessionThread {
    pub messages: Vec<MessageWithParts>,
    part_index: HashMap<PartId, (usize, usize)>,
}

#[derive(Debug, Clone)]
pub struct AppModel {
    pub connection: ConnectionStatus,
    pub sessions: Vec<Session>,
    pub active_session_id: Option<SessionId>,
    pub threads: HashMap<SessionId, SessionThread>,
    pub statuses: HashMap<SessionId, SessionStatus>,
    pub permissions: HashMap<PermissionId, PermissionRequest>,
    pub questions: HashMap<QuestionId, QuestionRequest>,
    pub providers: Option<ProviderListResult>,
    pub ptys: HashMap<PtyId, PtyInfo>,
}

impl Default for AppModel {
    fn default() -> Self {
        Self {
            connection: ConnectionStatus::Disconnected,
            sessions: Vec::new(),
            active_session_id: None,
            threads: HashMap::new(),
            statuses: HashMap::new(),
            permissions: HashMap::new(),
            questions: HashMap::new(),
            providers: None,
            ptys: HashMap::new(),
        }
    }
}

impl AppModel {
    pub fn apply_event(&mut self, event: OpenCodeEvent) {
        match event {
            OpenCodeEvent::SessionCreated { info, .. }
            | OpenCodeEvent::SessionUpdated { info, .. } => self.upsert_session(info),
            OpenCodeEvent::SessionDeleted { session_id, .. } => {
                self.sessions.retain(|session| session.id != session_id);
                self.threads.remove(&session_id);
                if self.active_session_id.as_deref() == Some(&session_id) {
                    self.active_session_id =
                        self.sessions.first().map(|session| session.id.clone());
                }
            }
            OpenCodeEvent::MessageUpdated { session_id, info } => {
                self.thread_mut(&session_id).upsert_message_info(info)
            }
            OpenCodeEvent::MessageRemoved {
                session_id,
                message_id,
            } => self.thread_mut(&session_id).remove_message(&message_id),
            OpenCodeEvent::MessagePartUpdated {
                session_id, part, ..
            } => self.thread_mut(&session_id).upsert_part(part),
            OpenCodeEvent::MessagePartRemoved {
                session_id,
                part_id,
                ..
            } => self.thread_mut(&session_id).remove_part(&part_id),
            OpenCodeEvent::MessagePartDelta {
                session_id,
                part_id,
                field,
                delta,
                ..
            } => self
                .thread_mut(&session_id)
                .append_part_delta(&part_id, &field, &delta),
            OpenCodeEvent::SessionStatus { session_id, status } => {
                self.statuses.insert(session_id, status);
            }
            OpenCodeEvent::SessionIdle { session_id } => {
                self.statuses.insert(session_id, SessionStatus::Idle);
            }
            OpenCodeEvent::PermissionAsked(request) => {
                self.permissions.insert(request.id.clone(), request);
            }
            OpenCodeEvent::PermissionReplied { request_id, .. } => {
                self.permissions.remove(&request_id);
            }
            OpenCodeEvent::QuestionAsked(request) => {
                self.questions.insert(request.id.clone(), request);
            }
            OpenCodeEvent::QuestionReplied { request_id, .. }
            | OpenCodeEvent::QuestionRejected { request_id, .. } => {
                self.questions.remove(&request_id);
            }
            OpenCodeEvent::PtyCreated { info } | OpenCodeEvent::PtyUpdated { info } => {
                self.ptys.insert(info.id.clone(), info);
            }
            OpenCodeEvent::PtyDeleted { id } | OpenCodeEvent::PtyExited { id, .. } => {
                self.ptys.remove(&id);
            }
            OpenCodeEvent::Unknown { .. } => {}
        }
    }

    pub fn upsert_session(&mut self, session: Session) {
        if let Some(existing) = self
            .sessions
            .iter_mut()
            .find(|existing| existing.id == session.id)
        {
            *existing = session;
        } else {
            if self.active_session_id.is_none() {
                self.active_session_id = Some(session.id.clone());
            }
            self.sessions.insert(0, session);
        }
    }

    fn thread_mut(&mut self, session_id: &str) -> &mut SessionThread {
        self.threads.entry(session_id.to_string()).or_default()
    }
}

impl SessionThread {
    pub fn replace_messages(&mut self, messages: Vec<MessageWithParts>) {
        self.messages = messages;
        self.rebuild_index();
    }

    fn upsert_message_info(&mut self, info: MessageInfo) {
        if let Some(message) = self
            .messages
            .iter_mut()
            .find(|message| message.info.id == info.id)
        {
            message.info = info;
        } else {
            self.messages.push(MessageWithParts {
                info,
                parts: Vec::new(),
            });
        }
        self.rebuild_index();
    }

    fn remove_message(&mut self, message_id: &str) {
        self.messages
            .retain(|message| message.info.id != message_id);
        self.rebuild_index();
    }

    fn upsert_part(&mut self, part: Part) {
        let message_idx = self.ensure_message(&part.session_id, &part.message_id);
        if let Some((existing_message_idx, existing_part_idx)) =
            self.part_index.get(&part.id).copied()
        {
            self.messages[existing_message_idx].parts[existing_part_idx] = part;
        } else {
            self.messages[message_idx].parts.push(part);
        }
        self.rebuild_index();
    }

    fn remove_part(&mut self, part_id: &str) {
        for message in &mut self.messages {
            message.parts.retain(|part| part.id != part_id);
        }
        self.rebuild_index();
    }

    fn append_part_delta(&mut self, part_id: &str, field: &str, delta: &str) {
        let Some((message_idx, part_idx)) = self.part_index.get(part_id).copied() else {
            return;
        };
        let part = &mut self.messages[message_idx].parts[part_idx];
        if field == "text" {
            part.text.get_or_insert_with(String::new).push_str(delta);
        }
    }

    fn ensure_message(&mut self, session_id: &str, message_id: &str) -> usize {
        if let Some(index) = self
            .messages
            .iter()
            .position(|message| message.info.id == message_id)
        {
            index
        } else {
            self.messages.push(MessageWithParts {
                info: MessageInfo {
                    id: message_id.to_string(),
                    session_id: session_id.to_string(),
                    role: "assistant".to_string(),
                    extra: HashMap::new(),
                },
                parts: Vec::new(),
            });
            self.messages.len() - 1
        }
    }

    fn rebuild_index(&mut self) {
        self.part_index.clear();
        for (message_idx, message) in self.messages.iter().enumerate() {
            for (part_idx, part) in message.parts.iter().enumerate() {
                self.part_index
                    .insert(part.id.clone(), (message_idx, part_idx));
            }
        }
    }
}

#[derive(Clone)]
pub struct AppStore {
    model: Arc<RwLock<AppModel>>,
    changes: broadcast::Sender<()>,
}

impl Default for AppStore {
    fn default() -> Self {
        Self::new(AppModel::default())
    }
}

impl AppStore {
    pub fn new(model: AppModel) -> Self {
        let (changes, _) = broadcast::channel(128);
        Self {
            model: Arc::new(RwLock::new(model)),
            changes,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.changes.subscribe()
    }

    pub async fn snapshot(&self) -> AppModel {
        self.model.read().await.clone()
    }

    pub async fn set_connection(&self, status: ConnectionStatus) {
        self.model.write().await.connection = status;
        let _ = self.changes.send(());
    }

    pub async fn replace_bootstrap(
        &self,
        sessions: Vec<Session>,
        statuses: HashMap<SessionId, SessionStatus>,
        permissions: Vec<PermissionRequest>,
        questions: Vec<QuestionRequest>,
        providers: Option<ProviderListResult>,
    ) {
        let mut model = self.model.write().await;
        model.sessions = sessions;
        model.active_session_id = model.sessions.first().map(|session| session.id.clone());
        model.statuses = statuses;
        model.permissions = permissions
            .into_iter()
            .map(|request| (request.id.clone(), request))
            .collect();
        model.questions = questions
            .into_iter()
            .map(|request| (request.id.clone(), request))
            .collect();
        model.providers = providers;
        let _ = self.changes.send(());
    }

    pub async fn apply_event(&self, event: OpenCodeEvent) {
        self.model.write().await.apply_event(event);
        let _ = self.changes.send(());
    }

    pub async fn set_active_session(&self, session_id: Option<SessionId>) {
        self.model.write().await.active_session_id = session_id;
        let _ = self.changes.send(());
    }

    pub async fn upsert_session(&self, session: Session) {
        self.model.write().await.upsert_session(session);
        let _ = self.changes.send(());
    }

    pub async fn remove_session(&self, session_id: &str) {
        let mut model = self.model.write().await;
        model.sessions.retain(|session| session.id != session_id);
        model.threads.remove(session_id);
        model.statuses.remove(session_id);
        if model.active_session_id.as_deref() == Some(session_id) {
            model.active_session_id = model.sessions.first().map(|session| session.id.clone());
        }
        let _ = self.changes.send(());
    }

    pub async fn remove_permission(&self, request_id: &str) {
        self.model.write().await.permissions.remove(request_id);
        let _ = self.changes.send(());
    }
}
