// REQ-CORE-018

use serde::Serialize;

use crate::{
    cli::LookupKind,
    model::{Feature, Philosophy, Policy, Requirement},
    workspace::Workspace,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct EntitySummary {
    pub id: String,
    pub title: String,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum WorkspaceEntity<'a> {
    Philosophy(&'a Philosophy),
    Policy(&'a Policy),
    Requirement(&'a Requirement),
    Feature(&'a Feature),
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct WorkspaceLookup<'a> {
    workspace: &'a Workspace,
}

impl<'a> WorkspaceLookup<'a> {
    pub(crate) fn new(workspace: &'a Workspace) -> Self {
        Self { workspace }
    }

    pub(crate) fn entries(self, kind: LookupKind) -> Vec<EntitySummary> {
        match kind {
            LookupKind::Philosophy => self
                .workspace
                .philosophies
                .iter()
                .map(|item| EntitySummary {
                    id: item.id.clone(),
                    title: item.title.clone(),
                })
                .collect(),
            LookupKind::Policy => self
                .workspace
                .policies
                .iter()
                .map(|item| EntitySummary {
                    id: item.id.clone(),
                    title: item.title.clone(),
                })
                .collect(),
            LookupKind::Requirement => self
                .workspace
                .requirements
                .iter()
                .map(|item| EntitySummary {
                    id: item.id.clone(),
                    title: item.title.clone(),
                })
                .collect(),
            LookupKind::Feature => self
                .workspace
                .features
                .iter()
                .map(|item| EntitySummary {
                    id: item.id.clone(),
                    title: item.title.clone(),
                })
                .collect(),
        }
    }

    pub(crate) fn title_for(self, kind: LookupKind, id: &str) -> Option<&'a str> {
        match kind {
            LookupKind::Philosophy => self.philosophy(id).map(|item| item.title.as_str()),
            LookupKind::Policy => self.policy(id).map(|item| item.title.as_str()),
            LookupKind::Requirement => self.requirement(id).map(|item| item.title.as_str()),
            LookupKind::Feature => self.feature(id).map(|item| item.title.as_str()),
        }
    }

    pub(crate) fn philosophy(self, id: &str) -> Option<&'a Philosophy> {
        self.workspace
            .philosophies
            .iter()
            .find(|item| item.id == id)
    }

    pub(crate) fn policy(self, id: &str) -> Option<&'a Policy> {
        self.workspace.policies.iter().find(|item| item.id == id)
    }

    pub(crate) fn requirement(self, id: &str) -> Option<&'a Requirement> {
        self.workspace
            .requirements
            .iter()
            .find(|item| item.id == id)
    }

    pub(crate) fn feature(self, id: &str) -> Option<&'a Feature> {
        self.workspace.features.iter().find(|item| item.id == id)
    }

    pub(crate) fn find(self, id: &str) -> Option<WorkspaceEntity<'a>> {
        if id.starts_with("PHIL-") {
            return self.philosophy(id).map(WorkspaceEntity::Philosophy);
        }
        if id.starts_with("POL-") {
            return self.policy(id).map(WorkspaceEntity::Policy);
        }
        if id.starts_with("REQ-") {
            return self.requirement(id).map(WorkspaceEntity::Requirement);
        }
        if id.starts_with("FEAT-") {
            return self.feature(id).map(WorkspaceEntity::Feature);
        }

        None
    }
}
