# Envyr

Envyr is a tool to automagically package an application and run it in a sandboxed environment.
*This project is in active development and may break.*

Ever wanted to run a script from a git repo without having to clone, install dependencies etc?
Envyr does that for you. It can detect the language, install dependencies and run the project in a Sandboxed environment for you.

e.g 
```bash
envyr run git@github.com:tchaudhry91/python-sample-script.git --autogen -- https://blog.tux-sudo.com > my_blog.html
```
This command will fetch the repo, build a sandbox (docker/podman supported at the moment), and run the script!
Read more about the supported features below.


## Installation
This project can be installed with Cargo. More pre-built packages will be available soon.

## Usage
Envyr has built-in intelligence to run the following types of applications at the moment:

#### 1. Python Scripts

Envyr will automatically detect and run your python scripts.

**Detection**:
- If the project contains a .py file, it will be detected as a python script.
- If the project contains a requirements.txt file, it will be installed in the sandbox before execution.
- If a requirements.txt is not found, it will attempt to produce one using [pipreqs](https://pypi.org/project/pipreqs). 
- The entrypoint is detected via a `if __name__ == __main__` or a shebang statements. Ties are broken via a priority and can be overridden with the `-x` flag.

**Example**:
- Here is envyr running a python script from a public repository.
 ```bash
$ envyr run --autogen git@github.com:sivel/speedtest-cli.git                    

Retrieving speedtest.net configuration...
Testing from xyz (xyz.xyz.xyz.xyz)...
Retrieving speedtest.net server list...
Selecting best server based on ping...
^C
Cancelling...
```
The first run will clone the repo and build the sandbox. Subsequent runs would be near instant.

#### 2. Node JS Scripts
Envyr will automatically detect and run your node.js scripts.

**Detection**:
- The project needs to contain a package.json.
- This is used to install the dependencies and figure out the entrypoint (`main` from package.json)

#### 3. Shell Scripts

**Detection**:
- Based on Shebang.
- *Pending*: A way to detect dependencies. They can still be supplied manually while generating.


#### 4. More to come later..

### Configuration Options
```
$ envyr -h
A tool to automagically create 'executable' packages for your scripts.

Usage: envyr [OPTIONS] <COMMAND>

Commands:
  generate  Generate the associated meta files. Overwrites if re-run.
  alias     Subcommands for aliases.
  run       Run the package with the given executor.
  help      Print this message or the help of the given subcommand(s)

Options:
  -v, --verbose  Emit Envyr logs to stdout. Useful for debugging. But may spoil pipes.
  -h, --help     Print help
  -V, --version  Print version
```

**Running a Package**
```
$ envyr run -h
Run the package with the given executor.

Usage: envyr run [OPTIONS] <PROJECT_ROOT> [-- <ARGS>...]

Arguments:
  <PROJECT_ROOT>  The location to the project. Accepts, local filesystem path/git repos.
  [ARGS]...       

Options:
  -s, --sub-dir <SUB_DIR>          relative sub-directory to the project_root, useful if you're working with monorepos.
  -t, --tag <TAG>                  The tag of the package to run. Accepts git tags/commits. Defaults to latest. [default: latest]
      --refresh                    refresh code cache before running.
      --alias <ALIAS>              Upon successful completion, record this run command as an alias. To allow usage of `envyr run <alias>` in the future.
  -e, --executor <EXECUTOR>        [default: docker] [possible values: docker, nix, native]
      --autogen                    Attempt to automatically generate the package metadata before running. This overwrites existing metadata.
      --fs-map [<FS_MAP>...]       Mount the given directory as a volume. Format: host_dir:container_dir. Allows multiples. Only applicable on Docker Executor.
      --port-map [<PORT_MAP>...]   Map ports to host system, Format host_port:source_port. Allows multiples. Only applicable on Docker Executor.
      --env-map [<ENV_MAP>...]     Environment variables to pass through, leave value empty to pass through the value from the current environment. Format: 'key=value' or 'key' (passwthrough). Allows multiples.
  -n, --name <NAME>                
  -i, --interpreter <INTERPRETER>  
  -x, --entrypoint <ENTRYPOINT>    
  -t, --type <PTYPE>               [possible values: python, node, shell, other]
  -h, --help                       Print help
```

Most cases should be covered by autodetection. Use the overrides if `--autogen` does not work.


**Generating Package Metadata in Advance**
```
Generate the associated meta files. Overwrites if re-run.

Usage: envyr generate [OPTIONS] <PROJECT_ROOT>

Arguments:
  <PROJECT_ROOT>  The location to the project. Accepts, local filesystem path/git repos.

Options:
  -s, --sub-dir <SUB_DIR>          relative sub-directory to the project_root, useful if you're working with monorepos.
  -t, --tag <TAG>                  The tag of the package to run. Accepts git tags/commits. Defaults to latest. [default: latest]
      --refresh                    refresh code cache before running.
  -n, --name <NAME>                
  -i, --interpreter <INTERPRETER>  
  -x, --entrypoint <ENTRYPOINT>    
  -t, --type <PTYPE>               [possible values: python, node, shell, other]
  -h, --help                       Print help
```

The generate command is generally meant to be used by authors who can commit the `.envyr` folder generated by this command. This allows others to run this package with the optional (entrypoint/interpreter) overrides that the author desires by default.

**Aliasing**
You can generate aliases for common run commands to make them more ergonomic for regular use.
Pass the `--alias` flag to create a new alias on a successful run of a particular package.
```
$envyr run --alias sample --env-map MYVAR --autogen git@github.com:tchaudhry91/python-sample-script.git -- https://blog.tux-sudo.com

```
This will store the above command as an alias called `sample` which can run as follows:
```
$envyr run sample
```

The `args` are also stored with the alias but can be overriden if required.
```
$envyr run sample -- https://test.com
```

Aliases can be managed using the following:
```
$envyr alias -h        2 ↵
Subcommands for aliases.

Usage: envyr alias <COMMAND>

Commands:
  list    List all aliases.
  delete  Delete an existing alias.
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```


### Planned Features

- Only Docker/Podman are available as the sandbox enviroments at the moment. Add nix/native options too.
- More Languages
- Bash Script Dependency Detection.

See the issue tracker/project board for more.
