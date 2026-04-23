fn build_unarchived_conversation_overview_payload(
    state: &AppState,
    app_config: &AppConfig,
    data: &AppData,
) -> UnarchivedConversationOverviewUpdatedPayload {
    let unarchived_conversations = collect_unarchived_conversation_summaries(state, app_config, data);
    let preferred_conversation_id = unarchived_conversations
        .first()
        .map(|item| item.conversation_id.clone());
    UnarchivedConversationOverviewUpdatedPayload {
        unarchived_conversations,
        preferred_conversation_id,
    }
}

fn persist_runtime_state_only(
    state: &AppState,
    data: &AppData,
    reason: &str,
) -> Result<(), String> {
    let total_started_at = std::time::Instant::now();
    let runtime_state_started_at = std::time::Instant::now();
    let runtime_state = build_runtime_state_file(data);
    let runtime_state_build_elapsed_ms = runtime_state_started_at.elapsed().as_millis();
    let runtime_write_started_at = std::time::Instant::now();
    state_write_runtime_state_cached(state, &runtime_state)?;
    let runtime_write_elapsed_ms = runtime_write_started_at.elapsed().as_millis();
    eprintln!(
        "[会话持久化] 运行态定向写入耗时：总计={}ms，构建运行态={}ms，运行态落盘={}ms，reason={}",
        total_started_at.elapsed().as_millis(),
        runtime_state_build_elapsed_ms,
        runtime_write_elapsed_ms,
        reason
    );
    Ok(())
}

fn foreground_conversation_ids(data: &AppData) -> Vec<String> {
    data.conversations
        .iter()
        .filter(|conversation| {
            conversation_visible_in_foreground_lists(conversation)
                && conversation.summary.trim().is_empty()
        })
        .map(|conversation| conversation.id.clone())
        .collect()
}

fn persist_selected_conversations_and_runtime(
    state: &AppState,
    data: &AppData,
    conversation_ids: &[String],
    reason: &str,
) -> Result<(), String> {
    let total_started_at = std::time::Instant::now();
    let mut unique_ids = std::collections::HashSet::<String>::new();
    let target_ids = conversation_ids
        .iter()
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .filter(|item| unique_ids.insert(item.clone()))
        .collect::<Vec<_>>();
    let conversation_write_started_at = std::time::Instant::now();
    let mut written_count = 0usize;
    for conversation_id in &target_ids {
        let Some(conversation) = data.conversations.iter().find(|item| item.id == *conversation_id) else {
            continue;
        };
        conversation_service().persist_conversation(state, conversation)?;
        written_count += 1;
    }
    let conversation_write_elapsed_ms = conversation_write_started_at.elapsed().as_millis();
    let chat_index_started_at = std::time::Instant::now();
    let mut chat_index = state_read_chat_index_cached(state)?;
    for conversation_id in &target_ids {
        let Some(conversation) = data.conversations.iter().find(|item| item.id == *conversation_id) else {
            continue;
        };
        upsert_chat_index_conversation(&mut chat_index, conversation);
    }
    state_write_chat_index_cached(state, &chat_index)?;
    let chat_index_elapsed_ms = chat_index_started_at.elapsed().as_millis();
    let runtime_state_started_at = std::time::Instant::now();
    let runtime_state = build_runtime_state_file(data);
    let runtime_state_build_elapsed_ms = runtime_state_started_at.elapsed().as_millis();
    let runtime_write_started_at = std::time::Instant::now();
    state_write_runtime_state_cached(state, &runtime_state)?;
    let runtime_write_elapsed_ms = runtime_write_started_at.elapsed().as_millis();
    eprintln!(
        "[会话持久化] 定向会话+运行态写入耗时：总计={}ms，会话写入={}ms(count={})，聊天索引写入={}ms，构建运行态={}ms，运行态落盘={}ms，reason={}",
        total_started_at.elapsed().as_millis(),
        conversation_write_elapsed_ms,
        written_count,
        chat_index_elapsed_ms,
        runtime_state_build_elapsed_ms,
        runtime_write_elapsed_ms,
        reason
    );
    Ok(())
}

fn persist_removed_and_selected_conversations_and_runtime(
    state: &AppState,
    data: &AppData,
    removed_conversation_ids: &[String],
    conversation_ids: &[String],
    reason: &str,
) -> Result<(), String> {
    let total_started_at = std::time::Instant::now();
    let mut unique_removed_ids = std::collections::HashSet::<String>::new();
    let removed_ids = removed_conversation_ids
        .iter()
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .filter(|item| unique_removed_ids.insert(item.clone()))
        .collect::<Vec<_>>();
    let conversation_delete_started_at = std::time::Instant::now();
    let mut deleted_count = 0usize;
    for conversation_id in &removed_ids {
        conversation_service().delete_conversation(state, conversation_id)?;
        deleted_count += 1;
    }
    let conversation_delete_elapsed_ms = conversation_delete_started_at.elapsed().as_millis();

    let mut unique_ids = std::collections::HashSet::<String>::new();
    let target_ids = conversation_ids
        .iter()
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .filter(|item| unique_ids.insert(item.clone()))
        .collect::<Vec<_>>();
    let conversation_write_started_at = std::time::Instant::now();
    let mut written_count = 0usize;
    for conversation_id in &target_ids {
        let Some(conversation) = data.conversations.iter().find(|item| item.id == *conversation_id) else {
            continue;
        };
        conversation_service().persist_conversation(state, conversation)?;
        written_count += 1;
    }
    let conversation_write_elapsed_ms = conversation_write_started_at.elapsed().as_millis();

    let chat_index_started_at = std::time::Instant::now();
    let mut chat_index = state_read_chat_index_cached(state)?;
    for conversation_id in &removed_ids {
        remove_chat_index_conversation(&mut chat_index, conversation_id);
    }
    for conversation_id in &target_ids {
        let Some(conversation) = data.conversations.iter().find(|item| item.id == *conversation_id) else {
            continue;
        };
        upsert_chat_index_conversation(&mut chat_index, conversation);
    }
    state_write_chat_index_cached(state, &chat_index)?;
    let chat_index_elapsed_ms = chat_index_started_at.elapsed().as_millis();

    let runtime_state_started_at = std::time::Instant::now();
    let runtime_state = build_runtime_state_file(data);
    let runtime_state_build_elapsed_ms = runtime_state_started_at.elapsed().as_millis();
    let runtime_write_started_at = std::time::Instant::now();
    state_write_runtime_state_cached(state, &runtime_state)?;
    let runtime_write_elapsed_ms = runtime_write_started_at.elapsed().as_millis();

    eprintln!(
        "[会话持久化] 定向删改写入耗时：总计={}ms，删除会话={}ms(count={})，写入会话={}ms(count={})，聊天索引写入={}ms，构建运行态={}ms，运行态落盘={}ms，reason={}",
        total_started_at.elapsed().as_millis(),
        conversation_delete_elapsed_ms,
        deleted_count,
        conversation_write_elapsed_ms,
        written_count,
        chat_index_elapsed_ms,
        runtime_state_build_elapsed_ms,
        runtime_write_elapsed_ms,
        reason
    );
    Ok(())
}

fn persist_conversation_set_delta(
    state: &AppState,
    before: &[Conversation],
    after: &[Conversation],
) -> Result<(), String> {
    let total_started_at = std::time::Instant::now();
    let before_map_started_at = std::time::Instant::now();
    let before_map = before
        .iter()
        .map(|conversation| (conversation.id.clone(), conversation))
        .collect::<std::collections::HashMap<_, _>>();
    let before_map_elapsed_ms = before_map_started_at.elapsed().as_millis();
    let after_ids_started_at = std::time::Instant::now();
    let after_ids = after
        .iter()
        .map(|conversation| conversation.id.clone())
        .collect::<std::collections::HashSet<_>>();
    let after_ids_elapsed_ms = after_ids_started_at.elapsed().as_millis();

    let compare_and_write_started_at = std::time::Instant::now();
    let mut changed_count = 0usize;
    for conversation in after {
        let changed = before_map
            .get(&conversation.id)
            .map(|previous| serde_json::to_vec(previous).ok() != serde_json::to_vec(conversation).ok())
            .unwrap_or(true);
        if changed {
            changed_count += 1;
            conversation_service().persist_conversation(state, conversation)?;
        }
    }
    let compare_and_write_elapsed_ms = compare_and_write_started_at.elapsed().as_millis();

    let delete_started_at = std::time::Instant::now();
    let mut deleted_count = 0usize;
    for conversation in before {
        if !after_ids.contains(&conversation.id) {
            deleted_count += 1;
            conversation_service().delete_conversation(state, &conversation.id)?;
        }
    }
    let delete_elapsed_ms = delete_started_at.elapsed().as_millis();
    eprintln!(
        "[会话持久化] 会话集合增量耗时：总计={}ms，构建旧映射={}ms，构建新ID集合={}ms，比较并写会话={}ms，删除会话={}ms，before_count={}，after_count={}，changed_count={}，deleted_count={}",
        total_started_at.elapsed().as_millis(),
        before_map_elapsed_ms,
        after_ids_elapsed_ms,
        compare_and_write_elapsed_ms,
        delete_elapsed_ms,
        before.len(),
        after.len(),
        changed_count,
        deleted_count
    );

    Ok(())
}

fn persist_app_data_conversation_runtime_delta(
    state: &AppState,
    before: &AppData,
    after: &AppData,
) -> Result<(), String> {
    let total_started_at = std::time::Instant::now();
    let conversations_compare_started_at = std::time::Instant::now();
    let conversations_changed =
        serde_json::to_vec(&before.conversations).ok() != serde_json::to_vec(&after.conversations).ok();
    let conversations_compare_elapsed_ms = conversations_compare_started_at.elapsed().as_millis();
    let runtime_before_started_at = std::time::Instant::now();
    let runtime_before = build_runtime_state_file(before);
    let runtime_before_elapsed_ms = runtime_before_started_at.elapsed().as_millis();
    let runtime_after_started_at = std::time::Instant::now();
    let runtime_after = build_runtime_state_file(after);
    let runtime_after_elapsed_ms = runtime_after_started_at.elapsed().as_millis();
    let runtime_compare_started_at = std::time::Instant::now();
    let runtime_changed =
        serde_json::to_vec(&runtime_before).ok() != serde_json::to_vec(&runtime_after).ok();
    let runtime_compare_elapsed_ms = runtime_compare_started_at.elapsed().as_millis();

    let conversation_delta_started_at = std::time::Instant::now();
    if conversations_changed {
        persist_conversation_set_delta(state, &before.conversations, &after.conversations)?;
        let chat_index = build_chat_index_file(&after.conversations);
        state_write_chat_index_cached(state, &chat_index)?;
    }
    let conversation_delta_elapsed_ms = conversation_delta_started_at.elapsed().as_millis();
    let runtime_write_started_at = std::time::Instant::now();
    if runtime_changed {
        state_write_runtime_state_cached(state, &runtime_after)?;
    }
    let runtime_write_elapsed_ms = runtime_write_started_at.elapsed().as_millis();
    eprintln!(
        "[会话持久化] 运行态增量耗时：总计={}ms，会话列表比较={}ms，构建运行态(before)={}ms，构建运行态(after)={}ms，运行态比较={}ms，会话增量落盘+索引={}ms，运行态落盘={}ms，conversation_count_before={}，conversation_count_after={}，conversations_changed={}，runtime_changed={}",
        total_started_at.elapsed().as_millis(),
        conversations_compare_elapsed_ms,
        runtime_before_elapsed_ms,
        runtime_after_elapsed_ms,
        runtime_compare_elapsed_ms,
        conversation_delta_elapsed_ms,
        runtime_write_elapsed_ms,
        before.conversations.len(),
        after.conversations.len(),
        conversations_changed,
        runtime_changed
    );

    Ok(())
}

fn persist_single_conversation_runtime_fast(
    state: &AppState,
    data: &AppData,
    conversation_id: &str,
) -> Result<(), String> {
    let total_started_at = std::time::Instant::now();
    let cid = conversation_id.trim();
    if cid.is_empty() {
        return Ok(());
    }
    let conversation = data
        .conversations
        .iter()
        .find(|item| item.id == cid)
        .ok_or_else(|| format!("会话不存在，无法快速持久化: {cid}"))?;

    let conversation_write_started_at = std::time::Instant::now();
    conversation_service().persist_conversation(state, conversation)?;
    let conversation_write_elapsed_ms = conversation_write_started_at.elapsed().as_millis();

    let chat_index_started_at = std::time::Instant::now();
    let mut chat_index = state_read_chat_index_cached(state)?;
    upsert_chat_index_conversation(&mut chat_index, conversation);
    state_write_chat_index_cached(state, &chat_index)?;
    let chat_index_elapsed_ms = chat_index_started_at.elapsed().as_millis();

    let runtime_state_started_at = std::time::Instant::now();
    let runtime_state = build_runtime_state_file(data);
    state_write_runtime_state_cached(state, &runtime_state)?;
    let runtime_state_elapsed_ms = runtime_state_started_at.elapsed().as_millis();

    eprintln!(
        "[会话持久化] 单会话快速写入耗时：总计={}ms，会话写入={}ms，聊天索引写入={}ms，运行态写入={}ms，conversation_id={}，conversation_count={}",
        total_started_at.elapsed().as_millis(),
        conversation_write_elapsed_ms,
        chat_index_elapsed_ms,
        runtime_state_elapsed_ms,
        cid,
        data.conversations.len()
    );

    Ok(())
}

fn resolve_unarchived_conversation_index_with_fallback(
    data: &mut AppData,
    app_config: &AppConfig,
    effective_agent_id: &str,
    requested_conversation_id: Option<&str>,
) -> Result<usize, String> {
    if let Some(conversation_id) = requested_conversation_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        if let Some(idx) = data.conversations.iter().position(|item| {
            item.id == conversation_id
                && item.summary.trim().is_empty()
                && conversation_visible_in_foreground_lists(item)
        }) {
            return Ok(idx);
        }
        runtime_log_warn(format!(
            "[解析对话索引] 请求的conversation_id不存在，终止本次读取: '{}' (agent_id: '{}')",
            conversation_id, effective_agent_id
        ));
        return Err(format!(
            "Requested conversation not found: {conversation_id}"
        ));
    }

    if let Some(existing_idx) = main_conversation_index(data, effective_agent_id) {
        return Ok(existing_idx);
    }

    if let Some(existing_idx) = latest_active_conversation_index(data, "", effective_agent_id) {
        return Ok(existing_idx);
    }

    let api_config = resolve_selected_api_config(app_config, None)
        .ok_or_else(|| "No API config available".to_string())?;
    Ok(ensure_active_conversation_index(
        data,
        &api_config.id,
        effective_agent_id,
    ))
}

fn resolve_foreground_snapshot_target_index(
    input: &SwitchActiveConversationSnapshotInput,
    app_config: &AppConfig,
    data: &mut AppData,
) -> Result<usize, String> {
    let requested_conversation_id = input
        .conversation_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let effective_agent_id = input
        .agent_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| data.assistant_department_agent_id.clone());
    resolve_unarchived_conversation_index_with_fallback(
        data,
        app_config,
        &effective_agent_id,
        requested_conversation_id,
    )
}

fn build_foreground_conversation_snapshot_core(
    state: &AppState,
    data: &AppData,
    target_idx: usize,
    recent_limit: usize,
) -> Result<ForegroundConversationSnapshotCore, String> {
    let conversation = data
        .conversations
        .get(target_idx)
        .ok_or_else(|| "Unarchived conversation index out of bounds.".to_string())?;
    let total_messages = conversation.messages.len();
    let start = total_messages.saturating_sub(recent_limit);
    let messages = conversation.messages[start..].to_vec();
    Ok(ForegroundConversationSnapshotCore {
        conversation_id: conversation.id.clone(),
        messages,
        has_more_history: start > 0,
        runtime_state: unarchived_conversation_runtime_state(state, &conversation.id),
        current_todo: conversation_current_todo_text(conversation),
        current_todos: conversation.current_todos.clone(),
    })
}

fn build_foreground_conversation_snapshot_from_conversation(
    state: &AppState,
    conversation: &Conversation,
    recent_limit: usize,
) -> Result<ForegroundConversationSnapshotCore, String> {
    let total_messages = conversation.messages.len();
    let start = total_messages.saturating_sub(recent_limit);
    let messages = conversation.messages[start..].to_vec();
    Ok(ForegroundConversationSnapshotCore {
        conversation_id: conversation.id.clone(),
        messages,
        has_more_history: start > 0,
        runtime_state: unarchived_conversation_runtime_state(state, &conversation.id),
        current_todo: conversation_current_todo_text(conversation),
        current_todos: conversation.current_todos.clone(),
    })
}

fn resolve_rewind_target_conversation_index(
    data: &AppData,
    requested_agent_id: &str,
    requested_conversation_id: Option<&str>,
) -> Result<usize, String> {
    if let Some(conversation_id) = requested_conversation_id {
        data.conversations
            .iter()
            .position(|item| {
                item.id == conversation_id
                    && item.summary.trim().is_empty()
                    && conversation_visible_in_foreground_lists(item)
            })
            .ok_or_else(|| {
                format!("Target conversation not found or unavailable, conversationId={conversation_id}")
            })
    } else {
        latest_active_conversation_index(data, "", requested_agent_id)
            .ok_or_else(|| "No conversation found for current agent.".to_string())
    }
}


impl ConversationService {
    fn persist_runtime_state_snapshot(
        &self,
        state: &AppState,
        data: &AppData,
        reason: &str,
    ) -> Result<(), String> {
        persist_runtime_state_only(state, data, reason)
    }

    fn persist_selected_conversations_snapshot(
        &self,
        state: &AppState,
        data: &AppData,
        conversation_ids: &[String],
        reason: &str,
    ) -> Result<(), String> {
        persist_selected_conversations_and_runtime(state, data, conversation_ids, reason)
    }

    fn persist_single_conversation_runtime_snapshot(
        &self,
        state: &AppState,
        data: &AppData,
        conversation_id: &str,
    ) -> Result<(), String> {
        persist_single_conversation_runtime_fast(state, data, conversation_id)
    }

    fn read_assistant_memory_context(
        &self,
        app_state: &AppState,
    ) -> Result<AssistantMemoryContext, String> {
        let guard = app_state
            .conversation_lock
            .lock()
            .map_err(|err| format!("Failed to lock state mutex at {}:{} {}: {err}", file!(), line!(), module_path!()))?;
        let data = state_read_app_data_cached(app_state)?;
        let assistant_department_agent_id = data.assistant_department_agent_id.clone();
        let owner_agent_id = data
            .agents
            .iter()
            .find(|a| {
                a.id == assistant_department_agent_id
                    && !a.is_built_in_user
                    && a.private_memory_enabled
            })
            .map(|a| a.id.clone());
        let private_memory_enabled = data
            .agents
            .iter()
            .find(|a| a.id == assistant_department_agent_id)
            .map(|a| a.private_memory_enabled)
            .unwrap_or(false);
        drop(guard);
        Ok(AssistantMemoryContext {
            owner_agent_id,
            assistant_department_agent_id,
            private_memory_enabled,
        })
    }


    fn persist_conversation_with_chat_index(
        &self,
        state: &AppState,
        conversation: &Conversation,
    ) -> Result<(), String> {
        state_schedule_conversation_persist(state, conversation, true).map(|_| ())
    }

    fn persist_conversation(
        &self,
        state: &AppState,
        conversation: &Conversation,
    ) -> Result<(), String> {
        state_schedule_conversation_persist(state, conversation, false).map(|_| ())
    }

    fn delete_conversation(&self, state: &AppState, conversation_id: &str) -> Result<(), String> {
        state_schedule_conversation_delete(state, conversation_id, false).map(|_| ())
    }

    fn update_conversation_todos(
        &self,
        state: &AppState,
        conversation_id: &str,
        stored_todos: &[ConversationTodoItem],
    ) -> Result<Option<ConversationTodosUpdateResult>, String> {
        let normalized_conversation_id = conversation_id.trim();
        if normalized_conversation_id.is_empty() {
            return Ok(None);
        }
        let guard = state
            .conversation_lock
            .lock()
            .map_err(|err| format!("Failed to lock state mutex at {}:{} {}: {err}", file!(), line!(), module_path!()))?;
        let mut conversation = match state_read_conversation_cached(state, normalized_conversation_id) {
            Ok(conversation) => conversation,
            Err(err) => {
                runtime_log_debug(format!(
                    "[Todo] 读取会话失败，函数=update_conversation_todos，conversation_id={}，error={}",
                    normalized_conversation_id, err
                ));
                drop(guard);
                return Ok(None);
            }
        };
        if !conversation.summary.trim().is_empty() {
            drop(guard);
            return Ok(None);
        }
        if conversation.current_todos == stored_todos {
            drop(guard);
            return Ok(None);
        }
        conversation.current_todos = stored_todos.to_vec();
        let current_todo = conversation_current_todo_text(&conversation);
        state_schedule_conversation_persist(state, &conversation, false)?;
        drop(guard);
        Ok(Some(ConversationTodosUpdateResult { current_todo }))
    }
}
