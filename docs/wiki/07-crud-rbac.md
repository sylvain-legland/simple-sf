# CRUD Operations & RBAC Rules

## CRUD Operations

| ID | Entity | Op | Endpoint | FFI | Feature |
|:---|:---|:---|:---|:---|:---|
| CRUD-SSF-001 | project | CREATE | /api/projects | sf_create_project | FT-SSF-003 |
| CRUD-SSF-002 | project | READ | /api/projects | sf_list_projects | FT-SSF-003 |
| CRUD-SSF-003 | project | READ | /api/projects/:id |  | FT-SSF-003 |
| CRUD-SSF-004 | project | UPDATE | /api/projects/:id |  | FT-SSF-003 |
| CRUD-SSF-005 | project | DELETE | /api/projects/:id | sf_delete_project | FT-SSF-003 |
| CRUD-SSF-006 | mission | CREATE |  | sf_start_mission | FT-SSF-004 |
| CRUD-SSF-007 | mission | READ |  | sf_mission_status | FT-SSF-004 |
| CRUD-SSF-008 | chat_session | CREATE | /api/chat/sessions |  | FT-SSF-001 |
| CRUD-SSF-009 | chat_message | CREATE | /api/chat/sessions/:id/message | sf_jarvis_discuss | FT-SSF-001 |
| CRUD-SSF-010 | chat_history | READ | /api/chat/sessions/:id/history | sf_load_discussion_history | FT-SSF-009 |
| CRUD-SSF-011 | ideation_session | CREATE | /api/ideation/sessions | sf_start_ideation | FT-SSF-006 |
| CRUD-SSF-012 | ideation_session | READ | /api/ideation/sessions |  | FT-SSF-006 |
| CRUD-SSF-013 | ideation_session | READ | /api/ideation/sessions/:id |  | FT-SSF-006 |
| CRUD-SSF-014 | agent | READ |  | sf_list_agents | FT-SSF-010 |
| CRUD-SSF-015 | workflow | READ |  | sf_list_workflows | FT-SSF-020 |
| CRUD-SSF-016 | provider | READ | /api/providers |  | FT-SSF-005 |
| CRUD-SSF-017 | provider | UPDATE |  | sf_configure_llm | FT-SSF-005 |
| CRUD-SSF-018 | user | CREATE | /api/auth/register |  | FT-SSF-024 |
| CRUD-SSF-019 | session | CREATE | /api/auth/login |  | FT-SSF-024 |
| CRUD-SSF-020 | user | READ | /api/auth/me |  | FT-SSF-024 |
| CRUD-SSF-021 | benchmark | CREATE |  | sf_run_bench | FT-SSF-021 |

## RBAC Rules

| ID | Resource | Action | Roles | Feature |
|:---|:---|:---|:---|:---|
| RBAC-SSF-001 | project | create | admin,lead,developer | FT-SSF-003 |
| RBAC-SSF-002 | project | read | all | FT-SSF-003 |
| RBAC-SSF-003 | project | update | admin,lead,developer | FT-SSF-003 |
| RBAC-SSF-004 | project | delete | admin,lead | FT-SSF-003 |
| RBAC-SSF-005 | mission | start | admin,lead,developer | FT-SSF-004 |
| RBAC-SSF-006 | mission | read | all | FT-SSF-004 |
| RBAC-SSF-007 | chat | send | all | FT-SSF-001 |
| RBAC-SSF-008 | chat | read_history | all | FT-SSF-009 |
| RBAC-SSF-009 | ideation | create | admin,lead,developer,po | FT-SSF-006 |
| RBAC-SSF-010 | ideation | read | all | FT-SSF-006 |
| RBAC-SSF-011 | agent | read | all | FT-SSF-010 |
| RBAC-SSF-012 | provider | configure | admin | FT-SSF-005 |
| RBAC-SSF-013 | provider | read | all | FT-SSF-005 |
| RBAC-SSF-014 | guard | read | admin,lead,security | FT-SSF-011 |
| RBAC-SSF-015 | sandbox | execute | admin,lead,developer | FT-SSF-012 |
| RBAC-SSF-016 | git | push | admin,lead,developer | FT-SSF-016 |
| RBAC-SSF-017 | export | zip | all | FT-SSF-017 |
| RBAC-SSF-018 | bench | run | admin,lead,qa | FT-SSF-021 |
| RBAC-SSF-019 | auth | register | public | FT-SSF-024 |
| RBAC-SSF-020 | auth | login | public | FT-SSF-024 |
