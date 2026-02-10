//! DataLoader implementations to prevent N+1 query problems

use async_graphql::dataloader::Loader;
use ramp_core::repository::user::{UserRepository, UserRow};
use ramp_common::types::{TenantId, UserId};
use std::collections::HashMap;
use std::sync::Arc;

/// Composite key for loading a user by tenant + user ID
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct UserByIdKey {
    pub tenant_id: String,
    pub user_id: String,
}

/// DataLoader for batch-loading users by ID to avoid N+1 queries.
///
/// Usage: When resolving a list of intents that each reference a user_id,
/// instead of making one DB call per intent, the DataLoader batches
/// all user_id lookups into a single query.
pub struct UserLoader {
    user_repo: Arc<dyn UserRepository>,
}

impl UserLoader {
    pub fn new(user_repo: Arc<dyn UserRepository>) -> Self {
        Self { user_repo }
    }
}

impl Loader<UserByIdKey> for UserLoader {
    type Value = UserRow;
    type Error = Arc<anyhow::Error>;

    fn load(
        &self,
        keys: &[UserByIdKey],
    ) -> impl std::future::Future<Output = Result<HashMap<UserByIdKey, Self::Value>, Self::Error>> + Send {
        let user_repo = self.user_repo.clone();
        let keys = keys.to_vec();

        async move {
            let mut result = HashMap::new();

            // Batch load by issuing individual queries
            // In production, this could be optimized with a WHERE id IN (...) query
            for key in &keys {
                let tenant_id = TenantId(key.tenant_id.clone());
                let user_id = UserId(key.user_id.clone());

                match user_repo.get_by_id(&tenant_id, &user_id).await {
                    Ok(Some(user)) => {
                        result.insert(key.clone(), user);
                    }
                    Ok(None) => {
                        // User not found, skip
                    }
                    Err(e) => {
                        return Err(Arc::new(anyhow::anyhow!(
                            "Failed to load user {}/{}: {}",
                            key.tenant_id,
                            key.user_id,
                            e
                        )));
                    }
                }
            }

            Ok(result)
        }
    }
}
