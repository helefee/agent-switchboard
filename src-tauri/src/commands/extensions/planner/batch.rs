use std::collections::BTreeMap;

use asb_core::extensions::contracts::{ExtensionBinding, PlanOperation};
use asb_core::extensions::plan::{ExtensionPlan, PlanPreconditions, PlannedOperation};

use super::deploy::required_id;
use super::document::{materialize_document_writes, DocumentBatch};
use super::{PlanRequest, PlanRequestOperation, Planner};
use crate::commands::error::CommandError;
use crate::commands::extensions::support::store_error;

impl Planner<'_> {
    pub(super) fn build(&self, request: &PlanRequest) -> Result<ExtensionPlan, CommandError> {
        if request.operations.is_empty() {
            return Err(CommandError::keyed(
                "extension-invalid", "errors.extops.batchPlanNoOperations",
                "批量计划没有包含任何资源操作",
            ));
        }
        let groups = self.group_requests(request)?;
        let mut batch = DocumentBatch::default();
        let mut operations = Vec::new();
        for group in groups {
            let mut combined: Option<PlannedOperation> = None;
            for entry in group {
                let mut planned = self.build_entry(entry, &mut batch)?;
                batch.advance(planned.targets.len());
                if let Some(operation) = combined.as_mut() {
                    if planned.targets.iter().any(|target| operation.targets.iter().any(|prior| {
                        prior.binding.id == target.binding.id || prior.target == target.target
                    })) {
                        return Err(overlapping_operation());
                    }
                    operation.targets.append(&mut planned.targets);
                } else {
                    combined = Some(planned);
                }
            }
            if let Some(operation) = combined {
                operations.push(operation);
            }
        }
        materialize_document_writes(&mut batch, &mut operations);
        Ok(ExtensionPlan {
            plan_id: String::new(), created_at: String::new(), operations,
            preconditions: PlanPreconditions {
                expires_at: String::new(), generation: 0, secret_digests: BTreeMap::new(),
            },
        })
    }

    fn group_requests<'a>(
        &self, request: &'a PlanRequest,
    ) -> Result<Vec<Vec<&'a PlanRequestOperation>>, CommandError> {
        let bindings = self.store.list_bindings().map_err(store_error)?;
        let mut groups: Vec<Vec<&PlanRequestOperation>> = Vec::new();
        let mut indexes: BTreeMap<String, usize> = BTreeMap::new();
        for entry in &request.operations {
            let definition_id = request_definition(entry, &bindings)?;
            if let Some(index) = indexes.get(&definition_id) {
                let group = &mut groups[*index];
                if group[0].operation != entry.operation
                    || entry.operation == PlanOperation::Update
                    || group.iter().any(|prior| entry.binding_id.is_some()
                        && prior.binding_id == entry.binding_id)
                {
                    return Err(overlapping_operation());
                }
                group.push(entry);
            } else {
                indexes.insert(definition_id, groups.len());
                groups.push(vec![entry]);
            }
        }
        Ok(groups)
    }

    fn build_entry(
        &self, entry: &PlanRequestOperation, batch: &mut DocumentBatch,
    ) -> Result<PlannedOperation, CommandError> {
        match entry.operation {
            PlanOperation::Install => self.build_install(
                required_id(&entry.definition_id, "errors.extops.installPlanMissingId", "安装计划缺少扩展 id")?,
                &entry.targets, batch,
            ),
            PlanOperation::Update => {
                let id = required_id(&entry.definition_id, "errors.extops.updatePlanMissingId", "更新计划缺少扩展 id")?;
                let definition = self.store.get_definition(id).map_err(store_error)?
                    .ok_or_else(|| CommandError::keyed("extension-not-found",
                        "errors.extops.definitionNotFound", "扩展不存在或已被删除"))?;
                let bindings = self.store.list_bindings().map_err(store_error)?
                    .into_iter().filter(|binding| binding.resource_id == id).collect::<Vec<_>>();
                self.build_redeploy(&definition, &bindings, batch)
            }
            PlanOperation::Enable | PlanOperation::Disable | PlanOperation::Remove => self.build_binding_change(
                entry.operation,
                required_id(&entry.binding_id, "errors.extops.operationMissingBindingId", "该操作缺少绑定 id")?,
                entry.shared_settings, batch,
            ),
            PlanOperation::Restore | PlanOperation::Repair => Err(CommandError::keyed(
                "extension-invalid", "errors.extops.operationUseDedicatedCommand",
                "恢复和修复必须使用专用计划入口",
            )),
        }
    }
}

fn request_definition(
    entry: &PlanRequestOperation, bindings: &[ExtensionBinding],
) -> Result<String, CommandError> {
    match entry.operation {
        PlanOperation::Restore | PlanOperation::Repair => Err(CommandError::keyed(
            "extension-invalid", "errors.extops.operationUseDedicatedCommand",
            "恢复和修复必须使用专用计划入口",
        )),
        PlanOperation::Enable | PlanOperation::Disable | PlanOperation::Remove => {
            let id = required_id(&entry.binding_id, "errors.extops.operationMissingBindingId", "该操作缺少绑定 id")?;
            bindings.iter().find(|binding| binding.id == id)
                .map(|binding| binding.resource_id.clone()).ok_or_else(|| CommandError::keyed(
                    "extension-not-found", "errors.extops.bindingNotFound", "绑定不存在或已被移除",
                ))
        }
        _ => Ok(required_id(&entry.definition_id,
            "errors.extops.operationMissingDefinitionId", "该操作缺少扩展 id")?.to_string()),
    }
}

fn overlapping_operation() -> CommandError {
    CommandError::keyed("extension-conflict", "errors.extops.overlappingOperationInBatch",
        "同一扩展的批量操作必须一致，且不能重复操作同一绑定或目标")
}

#[cfg(test)]
mod tests;
