use asb_core::extensions::contracts::{ExtensionBinding, ExtensionDefinition, ExtensionPayload};

/// Called by producers that render content, after successful preparation.
/// The plan owns the complete post-apply binding; apply never guesses which
/// definition revision was actually deployed from an operation label.
pub(super) fn deployed_binding(
    definition: &ExtensionDefinition, binding: &ExtensionBinding,
) -> ExtensionBinding {
    let mut next = binding.clone();
    let current_content = match &definition.payload {
        ExtensionPayload::Mcp(_) => true,
        ExtensionPayload::Skill(skill) => binding.locked_digest.as_deref()
            .is_none_or(|locked| locked == skill.content_digest),
    };
    if current_content { next.last_applied_revision = Some(definition.revision); }
    next
}

#[cfg(test)]
mod tests;
