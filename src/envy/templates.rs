pub static TEMPLATE_DOCKERFILE: &str = r#"
# Envy Base
{{#if (eq ptype "Python")}}
FROM python:alpine
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

WORKDIR /app
# Install Type Specific Deps
{{#if type_reqs}}
{{#if (eq ptype "Python")}}
ADD ./requirements.txt /app/requirements.txt
RUN pip install -r requirements.txt
{{/if}}
{{#if (eq ptype "Node")}}
ADD ./package.json /app/package.json
RUN npm install
{{/if}}
{{/if}}

ADD . /app
ENTRYPOINT ["{{interpreter}}", "{{entrypoint}}"]
"#;
