pub static TEMPLATE_DOCKERFILE: &str = r#"
{{#if (eq ptype "Python")}}
FROM python:alpine
{{/if}}
{{#if (eq ptype "Node")}}
FROM node:alpine
{{/if}}
RUN apk add --no-cache ca-certificates
{{#if os_deps}}
RUN apk add --no-cache {{#each os_deps}} {{this}} {{/each}}
{{/if}}
WORKDIR /app
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
