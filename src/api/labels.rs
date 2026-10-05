//! Label endpoints.

use reqwest::Method;
use serde::Serialize;
use uuid::Uuid;

use crate::client::Client;
use crate::error::Result;
use crate::models::{Label, LabelAssociation, LabelItems, LabelsResponse};
use crate::rate_limit::{Concurrency, RateClass};
use crate::types::OkResponse;

#[derive(Serialize)]
struct CreateLabelBody<'a> {
    name: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    color: Option<&'a str>,
}

#[derive(Serialize)]
struct UpdateLabelBody<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    color: Option<&'a str>,
}

#[derive(Serialize)]
struct AssociateLabelBody<'a> {
    name: &'a str,
}

#[derive(Serialize)]
struct ThumbnailBody {
    asset_id: Option<Uuid>,
}

impl Client {
    /// Lists a project's labels.
    ///
    /// `GET /projects/{project_id}/labels`
    pub async fn list_labels(&self, project_id: impl Into<Uuid>) -> Result<LabelsResponse> {
        let project_id = project_id.into();
        self.send_json(
            RateClass::Tier2,
            "GET /projects/{project_id}/labels",
            Concurrency::NONE,
            || self.request(Method::GET, &format!("projects/{project_id}/labels")),
        )
        .await
    }

    /// Creates a label. A project may have at most 10,000 labels.
    ///
    /// `POST /projects/{project_id}/labels`
    pub async fn create_label(
        &self,
        project_id: impl Into<Uuid>,
        name: impl AsRef<str>,
        color: Option<&str>,
    ) -> Result<Label> {
        let project_id = project_id.into();
        let name = name.as_ref().to_string();
        let color = color.map(str::to_owned);
        self.send_json(
            RateClass::Writes,
            "POST /projects/{project_id}/labels",
            Concurrency::NONE,
            || {
                self.request(Method::POST, &format!("projects/{project_id}/labels"))
                    .json(&CreateLabelBody {
                        name: &name,
                        color: color.as_deref(),
                    })
            },
        )
        .await
    }

    /// Updates a label's name and/or color.
    ///
    /// `PATCH /labels/{label_id}`
    pub async fn update_label(
        &self,
        label_id: impl Into<Uuid>,
        name: Option<&str>,
        color: Option<&str>,
    ) -> Result<Label> {
        let label_id = label_id.into();
        let name = name.map(str::to_owned);
        let color = color.map(str::to_owned);
        self.send_json(
            RateClass::Writes,
            "PATCH /labels/{label_id}",
            Concurrency::NONE,
            || {
                self.request(Method::PATCH, &format!("labels/{label_id}"))
                    .json(&UpdateLabelBody {
                        name: name.as_deref(),
                        color: color.as_deref(),
                    })
            },
        )
        .await
    }

    /// Deletes a label.
    ///
    /// `DELETE /labels/{label_id}`
    pub async fn delete_label(&self, label_id: impl Into<Uuid>) -> Result<OkResponse> {
        let label_id = label_id.into();
        self.send_ack(
            RateClass::Writes,
            "DELETE /labels/{label_id}",
            Concurrency::NONE,
            || self.request(Method::DELETE, &format!("labels/{label_id}")),
        )
        .await
    }

    /// Lists the sprites, sounds and saved animations tagged with a label.
    ///
    /// `GET /labels/{label_id}/items`
    pub async fn label_items(&self, label_id: impl Into<Uuid>) -> Result<LabelItems> {
        let label_id = label_id.into();
        self.send_json(
            RateClass::Tier2,
            "GET /labels/{label_id}/items",
            Concurrency::NONE,
            || self.request(Method::GET, &format!("labels/{label_id}/items")),
        )
        .await
    }

    /// Sets or clears a label's cover thumbnail.
    ///
    /// `POST /labels/{label_id}/thumbnail`
    pub async fn set_label_thumbnail(
        &self,
        label_id: impl Into<Uuid>,
        asset_id: Option<Uuid>,
    ) -> Result<Label> {
        let label_id = label_id.into();
        self.send_json(
            RateClass::Writes,
            "POST /labels/{label_id}/thumbnail",
            Concurrency::NONE,
            || {
                self.request(Method::POST, &format!("labels/{label_id}/thumbnail"))
                    .json(&ThumbnailBody { asset_id })
            },
        )
        .await
    }

    /// Adds a label to a sprite asset.
    ///
    /// `POST /assets/{asset_id}/labels`
    pub async fn associate_asset_label(
        &self,
        asset_id: impl Into<Uuid>,
        name: impl AsRef<str>,
    ) -> Result<LabelAssociation> {
        let asset_id = asset_id.into();
        let name = name.as_ref().to_string();
        self.send_json(
            RateClass::Writes,
            "POST /assets/{asset_id}/labels",
            Concurrency::NONE,
            || {
                self.request(Method::POST, &format!("assets/{asset_id}/labels"))
                    .json(&AssociateLabelBody { name: &name })
            },
        )
        .await
    }

    /// Removes a label from a sprite asset.
    ///
    /// `DELETE /assets/{asset_id}/labels?name={name}`
    pub async fn remove_asset_label(
        &self,
        asset_id: impl Into<Uuid>,
        name: impl AsRef<str>,
    ) -> Result<LabelAssociation> {
        let asset_id = asset_id.into();
        let name = name.as_ref().to_string();
        self.send_json(
            RateClass::Writes,
            "DELETE /assets/{asset_id}/labels",
            Concurrency::NONE,
            || {
                self.request(Method::DELETE, &format!("assets/{asset_id}/labels"))
                    .query(&[("name", name.as_str())])
            },
        )
        .await
    }

    /// Dismisses a suggested label for a sprite asset.
    ///
    /// `DELETE /assets/{asset_id}/label-suggestions?name={name}`
    pub async fn dismiss_asset_label_suggestion(
        &self,
        asset_id: impl Into<Uuid>,
        name: impl AsRef<str>,
    ) -> Result<OkResponse> {
        let asset_id = asset_id.into();
        let name = name.as_ref().to_string();
        self.send_ack(
            RateClass::Writes,
            "DELETE /assets/{asset_id}/label-suggestions",
            Concurrency::NONE,
            || {
                self.request(
                    Method::DELETE,
                    &format!("assets/{asset_id}/label-suggestions"),
                )
                .query(&[("name", name.as_str())])
            },
        )
        .await
    }

    /// Adds a label to a sound asset.
    ///
    /// `POST /sound-assets/{asset_id}/labels`
    pub async fn associate_sound_label(
        &self,
        asset_id: impl Into<Uuid>,
        name: impl AsRef<str>,
    ) -> Result<LabelAssociation> {
        let asset_id = asset_id.into();
        let name = name.as_ref().to_string();
        self.send_json(
            RateClass::Writes,
            "POST /sound-assets/{asset_id}/labels",
            Concurrency::NONE,
            || {
                self.request(Method::POST, &format!("sound-assets/{asset_id}/labels"))
                    .json(&AssociateLabelBody { name: &name })
            },
        )
        .await
    }

    /// Removes a label from a sound asset.
    ///
    /// `DELETE /sound-assets/{asset_id}/labels?name={name}`
    pub async fn remove_sound_label(
        &self,
        asset_id: impl Into<Uuid>,
        name: impl AsRef<str>,
    ) -> Result<LabelAssociation> {
        let asset_id = asset_id.into();
        let name = name.as_ref().to_string();
        self.send_json(
            RateClass::Writes,
            "DELETE /sound-assets/{asset_id}/labels",
            Concurrency::NONE,
            || {
                self.request(Method::DELETE, &format!("sound-assets/{asset_id}/labels"))
                    .query(&[("name", name.as_str())])
            },
        )
        .await
    }

    /// Dismisses a suggested label for a sound asset.
    ///
    /// `DELETE /sound-assets/{asset_id}/label-suggestions?name={name}`
    pub async fn dismiss_sound_label_suggestion(
        &self,
        asset_id: impl Into<Uuid>,
        name: impl AsRef<str>,
    ) -> Result<OkResponse> {
        let asset_id = asset_id.into();
        let name = name.as_ref().to_string();
        self.send_ack(
            RateClass::Writes,
            "DELETE /sound-assets/{asset_id}/label-suggestions",
            Concurrency::NONE,
            || {
                self.request(
                    Method::DELETE,
                    &format!("sound-assets/{asset_id}/label-suggestions"),
                )
                .query(&[("name", name.as_str())])
            },
        )
        .await
    }

    /// Adds a label to a saved animation.
    ///
    /// `POST /saved-animations/{id}/labels`
    pub async fn associate_saved_animation_label(
        &self,
        id: impl Into<Uuid>,
        name: impl AsRef<str>,
    ) -> Result<LabelAssociation> {
        let id = id.into();
        let name = name.as_ref().to_string();
        self.send_json(
            RateClass::Writes,
            "POST /saved-animations/{id}/labels",
            Concurrency::NONE,
            || {
                self.request(Method::POST, &format!("saved-animations/{id}/labels"))
                    .json(&AssociateLabelBody { name: &name })
            },
        )
        .await
    }

    /// Removes a label from a saved animation.
    ///
    /// `DELETE /saved-animations/{id}/labels?name={name}`
    pub async fn remove_saved_animation_label(
        &self,
        id: impl Into<Uuid>,
        name: impl AsRef<str>,
    ) -> Result<LabelAssociation> {
        let id = id.into();
        let name = name.as_ref().to_string();
        self.send_json(
            RateClass::Writes,
            "DELETE /saved-animations/{id}/labels",
            Concurrency::NONE,
            || {
                self.request(Method::DELETE, &format!("saved-animations/{id}/labels"))
                    .query(&[("name", name.as_str())])
            },
        )
        .await
    }
}
