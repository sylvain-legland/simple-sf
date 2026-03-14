# Ref: FT-SSF-025
# Infrastructure as Code for SF Simple

terraform {
  required_version = ">= 1.5"
  required_providers {
    docker = {
      source  = "kreuzwerker/docker"
      version = "~> 3.0"
    }
  }
}

variable "jwt_secret" {
  type      = string
  sensitive = true
}

variable "cors_origins" {
  type    = string
  default = "http://localhost:3000"
}

resource "docker_image" "sf_server" {
  name = "sf-simple-server:latest"
  build {
    context = "${path.module}/.."
  }
}

resource "docker_container" "sf_server" {
  name  = "sf-simple-server"
  image = docker_image.sf_server.image_id

  ports {
    internal = 8099
    external = 8099
  }

  env = [
    "JWT_SECRET=${var.jwt_secret}",
    "CORS_ORIGINS=${var.cors_origins}",
  ]

  restart = "unless-stopped"

  healthcheck {
    test     = ["CMD", "curl", "-f", "http://localhost:8099/health"]
    interval = "30s"
    timeout  = "10s"
    retries  = 3
  }
}

output "server_url" {
  value = "http://localhost:8099"
}
