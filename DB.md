# DB Tables

### data

| id | VARCHAR(36) |
| name | VARCHAR(255) |
| uptime | INT |
| os_release | VARCHAR(255) |
| memory | VARCHAR(255) |
| units | VARCHAR(255) |

### scripts

| id | varchar(36) | uuid |
| name | text |
| version | varchar(5) |
| script_context | text |
| output_regex | text |
| labels | json |
| required_labels | json |


### hosts 

| id | varchar(36) | uuid
| alias | text |
| labels | json |

### metrics

| id | varchar(36) |
| host_id | varchar(36) |
| name | varchar(36) |
| value | double |

### executions

| id | varchar(36) |
| ts | timestamp |
| host_id | varchar(36) |
| script_id | varchar(36) |
| output | text |