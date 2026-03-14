import Foundation

// Ref: FT-SSF-002

// MARK: - C FFI declarations (project management)

@_silgen_name("sf_create_project")
func _sf_create_project(_ name: UnsafePointer<CChar>?, _ desc: UnsafePointer<CChar>?, _ tech: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>?

@_silgen_name("sf_list_projects")
func _sf_list_projects() -> UnsafeMutablePointer<CChar>?

@_silgen_name("sf_delete_project")
func _sf_delete_project(_ id: UnsafePointer<CChar>?)

// MARK: - Project Operations

extension SFBridge {

    struct SFProject: Codable, Identifiable, Hashable {
        let id: String
        let name: String
        let description: String
        let tech: String
        let status: String
        let created_at: String
    }

    func createProject(name: String, description: String, tech: String) -> String? {
        var result: String?
        name.withCString { n in
            description.withCString { d in
                tech.withCString { t in
                    if let ptr = _sf_create_project(n, d, t) {
                        result = String(cString: ptr)
                        _sf_free_string(ptr)
                    }
                }
            }
        }
        return result
    }

    func listProjects() -> [SFProject] {
        guard let ptr = _sf_list_projects() else { return [] }
        let json = String(cString: ptr)
        _sf_free_string(ptr)
        guard let data = json.data(using: .utf8) else { return [] }
        return (try? JSONDecoder().decode([SFProject].self, from: data)) ?? []
    }

    func deleteProject(id: String) {
        id.withCString { ptr in
            _sf_delete_project(ptr)
        }
    }
}
