set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

app_name := "kwtypr"
default_profile := "release"
repo_root := justfile_directory()
desktop_dir := env_var("HOME") + "/.local/share/applications"

default:
	@just --list

build profile="debug":
	cargo build {{ if profile == "release" { "--release" } else { "" } }}

install profile=default_profile: (build profile)
	mkdir -p "{{desktop_dir}}"
	sed \
		-e 's|@exec@|{{repo_root}}/target/{{profile}}/{{app_name}}|g' \
		-e 's|@display_name@|kwtypr|g' \
		"{{repo_root}}/dist/{{app_name}}.desktop.in" > "{{desktop_dir}}/{{app_name}}.desktop"
	update-desktop-database "{{desktop_dir}}"
	@echo "Installed {{desktop_dir}}/{{app_name}}.desktop"
	@echo "Exec={{repo_root}}/target/{{profile}}/{{app_name}}"

run profile=default_profile: (install profile)
	"{{repo_root}}/target/{{profile}}/{{app_name}}"

uninstall:
	rm -f "{{desktop_dir}}/{{app_name}}.desktop"
	update-desktop-database "{{desktop_dir}}"
	@echo "Removed {{desktop_dir}}/{{app_name}}.desktop"
