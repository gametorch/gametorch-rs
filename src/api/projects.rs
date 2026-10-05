//! Project endpoints.

use reqwest::Method;
use serde::Serialize;

use crate::client::Client;
use crate::error::Result;
use crate::models::{Project, ProjectsResponse};
use crate::rate_limit::{Concurrency, RateClass};
use crate::types::OkResponse;

#[derive(Serialize)]
struct NameBody<'a> {
    name: &'a str,
}

impl Client {
    /// Lists the projects in the caller's scope.
    ///
    /// `GET /projects`
    pub async fn list_projects(&self) -> Result<ProjectsResponse> {
        self.send_json(RateClass::Tier2, "GET /projects", Concurrency::NONE, || {
            self.request(Method::GET, "projects")
        })
        .await
    }

    /// Creates a project owned by the caller's scope.
    ///
    /// `POST /projects`
    pub async fn create_project(&self, name: impl AsRef<str>) -> Result<Project> {
        let name = name.as_ref().to_string();
        self.send_json(
            RateClass::Writes,
            "POST /projects",
            Concurrency::NONE,
            || {
                self.request(Method::POST, "projects")
                    .json(&NameBody { name: &name })
            },
        )
        .await
    }

    /// Renames a project. The slug may change.
    ///
    /// `PATCH /projects/{slug}`
    pub async fn rename_project(
        &self,
        slug: impl AsRef<str>,
        name: impl AsRef<str>,
    ) -> Result<Project> {
        let slug = slug.as_ref().to_string();
        let name = name.as_ref().to_string();
        self.send_json(
            RateClass::Writes,
            "PATCH /projects/{slug}",
            Concurrency::NONE,
            || {
                self.request(Method::PATCH, &format!("projects/{slug}"))
                    .json(&NameBody { name: &name })
            },
        )
        .await
    }

    /// Permanently deletes a project and its generations and assets.
    ///
    /// `DELETE /projects/{slug}`
    pub async fn delete_project(&self, slug: impl AsRef<str>) -> Result<OkResponse> {
        let slug = slug.as_ref().to_string();
        self.send_ack(
            RateClass::Writes,
            "DELETE /projects/{slug}",
            Concurrency::NONE,
            || self.request(Method::DELETE, &format!("projects/{slug}")),
        )
        .await
    }
}
