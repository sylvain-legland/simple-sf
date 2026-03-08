#ifndef SF_ENGINE_H
#define SF_ENGINE_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// Callback: (agent_id, event_type, data)
typedef void (*SFEventCallback)(const char*, const char*, const char*);

// Init & config
void sf_init(const char* db_path);
void sf_set_callback(SFEventCallback cb);
void sf_configure_llm(const char* provider, const char* api_key, const char* base_url, const char* model);

// Projects
char* sf_create_project(const char* name, const char* description, const char* tech);
char* sf_list_projects(void);
void sf_delete_project(const char* id);

// Missions
char* sf_start_mission(const char* project_id, const char* brief);
char* sf_mission_status(const char* mission_id);

// Agents
char* sf_list_agents(void);

// Memory management
void sf_free_string(char* s);

#ifdef __cplusplus
}
#endif

#endif // SF_ENGINE_H
