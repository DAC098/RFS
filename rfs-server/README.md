# rfs-server

the actual file server executable.

## running

the server takes command line arguments and also takes a config file that will allow for more options to be specified. to see a list of available command line args run `rfs-server --help`.

config file options are as follows (JSON | Yaml):

```yaml
# the id of the server, used for creating ids in the database
id: 1
# the directory to store server data, can be relative or absolute
data: /etc/rfs/rfs-server
# ip address to bind the server to
ip: 0.0.0.0
# ip port to listen on
port: 8000

# template rendering options
templates:
  # directory where templates are stored
  directory: /etc/rfs/templates
  # specifies a dev option to always load templates from the file system
  dev_mode: false

# assets to be available for browser interfaces (not implemented)
# paths can be relative or absolute
assets:
  # list of files available. the key is the url for the request and the value
  # is the path to the file
  files:
    "/favicon.ico": /etc/rfs/assets/favicon.ico
  # list of directories to do file lookups. the key is the url for the request
  # and the value is the path to the directory
  directories:
    "/assets": /etc/rfs/assets

# the available options for security features
sec:
  # options specific to user session management
  session:
    # the hashing algorithm to use for session tokens
    hash: Blake3
    # stuff
    secure: false

# PostgreSQL database connection information
db:
  user: postgres
  password: password
  host: localhost
  port: 5432
  dbname: rfs
```