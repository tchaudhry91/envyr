pub static TEMPLATE_DOCKERFILE: &str = r#"
# Envyr Base
{{#if (eq ptype "Python")}}
FROM python:3.11-alpine
{{else}}
{{#if (eq ptype "Node")}}
FROM node:alpine
{{else}}
FROM alpine
{{/if}}
{{/if}}

# Base Deps
RUN apk add --no-cache ca-certificates bash

# Os Level Deps if any
{{#if os_deps}}
RUN apk add --no-cache {{#each os_deps}} {{this}} {{/each}}
{{/if}}

WORKDIR /envyr/app
# Install Type Specific Deps
{{#if type_reqs}}
{{#if (eq ptype "Python")}}
ADD ./requirements.txt /envyr/app/requirements.txt
RUN pip install -r requirements.txt
{{/if}}
{{#if (eq ptype "Node")}}
ADD ./package.json /envyr/app/package.json
RUN npm install
{{/if}}
{{/if}}

ADD . /envyr/app
ENTRYPOINT ["{{interpreter}}", "{{entrypoint}}"]
"#;

// To-Do
// Make modular per ptype later.
pub static DOCKER_IGNORE: &str = r#"
**/.git
**/.gitignore
**/node_modules
*.pyc
"#;
