# Skills Workspace

Skill 由本目录统一管理。
本说明参考 `docs/agent-skills-standard.md` 的规范。

## 目录规则

- 每个 skill 一个子目录
- 子目录名建议与 `SKILL.md` 中 `name` 一致
- 每个 skill 目录至少包含 `SKILL.md`
- 可选目录：
  - `scripts/`：可执行脚本
  - `references/`：按需加载的参考资料
  - `assets/`：模板/静态资源

## 推荐结构

```text
./
  your-skill/
    SKILL.md
    scripts/
    references/
    assets/
```

## SKILL.md 规范（重点）

`SKILL.md` 需要 YAML frontmatter + Markdown 正文：

- 必需字段：
  - `name`: 技能标识，建议小写、连字符
  - `description`: 说明技能做什么、何时触发
- 正文：写执行规则、步骤、示例、边界处理

## SKILL.md 最小示例

```md
---
name: your-skill
description: 处理 X 场景，适用于 Y 任务
---

# Instructions

1. 明确输入与目标。
2. 执行固定流程。
3. 输出结构化结果。
```

## 编写建议（来自 Agent Skills 标准）

- 前置元数据要可触发：`description` 里写清“何时使用”。
- 保持渐进式披露：
  - `SKILL.md` 放核心流程
  - 大量细节放 `references/`
- 一致性逻辑放脚本，不要完全依赖模型自由发挥。
- 正文尽量精炼，避免超长导致上下文膨胀。
