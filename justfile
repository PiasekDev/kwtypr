set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

app_name := "kwtypr"
default_profile := "release"
repo_root := justfile_directory()
default_prefix := env("HOME") + "/.local"
user_applications_dir := default_prefix + "/share/applications"

default:
	@just --list

build profile="debug":
	cargo build {{ if profile == "release" { "--release" } else { "" } }}

install profile=default_profile prefix=default_prefix: (build profile)
	install -Dm755 "{{repo_root}}/target/{{profile}}/{{app_name}}" "{{prefix}}/bin/{{app_name}}"
	mkdir -p "{{prefix}}/share/applications"
	sed \
		-e 's|@exec@|{{prefix}}/bin/{{app_name}}|g' \
		-e 's|@display_name@|kwtypr|g' \
		"{{repo_root}}/dist/{{app_name}}.desktop.in" > "{{prefix}}/share/applications/{{app_name}}.desktop"
	@just --justfile "{{repo_root}}/justfile" _refresh-desktop-cache "{{prefix}}/share/applications"
	@echo "Installed {{prefix}}/bin/{{app_name}}"
	@echo "Installed {{prefix}}/share/applications/{{app_name}}.desktop"

install-dev profile=default_profile: (build profile)
	mkdir -p "{{user_applications_dir}}"
	sed \
		-e 's|@exec@|{{repo_root}}/target/{{profile}}/{{app_name}}|g' \
		-e 's|@display_name@|kwtypr|g' \
		"{{repo_root}}/dist/{{app_name}}.desktop.in" > "{{user_applications_dir}}/{{app_name}}.desktop"
	@just --justfile "{{repo_root}}/justfile" _refresh-desktop-cache "{{user_applications_dir}}"
	@echo "Installed {{user_applications_dir}}/{{app_name}}.desktop"
	@echo "Exec={{repo_root}}/target/{{profile}}/{{app_name}}"

run profile=default_profile: (install-dev profile)
	"{{repo_root}}/target/{{profile}}/{{app_name}}"

uninstall prefix=default_prefix:
	rm -f "{{prefix}}/bin/{{app_name}}" "{{prefix}}/share/applications/{{app_name}}.desktop"
	@just --justfile "{{repo_root}}/justfile" _refresh-desktop-cache "{{prefix}}/share/applications"
	@echo "Removed {{prefix}}/bin/{{app_name}}"
	@echo "Removed {{prefix}}/share/applications/{{app_name}}.desktop"

[private]
_refresh-desktop-cache applications_dir:
	@if command -v kbuildsycoca6 >/dev/null; then \
		kbuildsycoca6 >/dev/null; \
	elif command -v kbuildsycoca5 >/dev/null; then \
		kbuildsycoca5 >/dev/null; \
	else \
		echo "warning: neither kbuildsycoca6 nor kbuildsycoca5 was found; KDE may not pick up desktop entry changes immediately" >&2; \
	fi
	@if [ -d "{{applications_dir}}" ] && command -v update-desktop-database >/dev/null; then \
		update-desktop-database "{{applications_dir}}" >/dev/null; \
	fi
