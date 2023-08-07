# rust-file-server (rfs)

pretty much as the name implies a file server written in rust. using this as a way to learn more about various things as well as  integrating with third party services such as AWS and Azure but also plan on using this for my own purposes.

planned features
- integration with third-party storage providers
    - Azure
    - AWS
    - BackBlaze
- event propagation
    - web-hooks
- secret management 
    - local 
    - Hashicorp Valut
    - Azure Key Vault
    - AWS Secrets Manager
- file versioning
- oauth support
- bot accounts for automation

a cli is available for interacting with the server and a browser interface is planned for the future.

## Setup

the server requires a PostgreSQL database to operate and store data. is an initialization script to setup a database that can be run manually or use `rfs-db` to run the initialization process for you.

once the database is setup the server can be run. tests have been run on Rocky and Ubuntu Linux thus far but don't foresee a reason that it will not work on other systems.

check the individual `README.md`'s for more information on how to run/use the sub-modules
