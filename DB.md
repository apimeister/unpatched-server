# DB Tables

## table structures

### data

| id | VARCHAR(36) | uuid
| name | VARCHAR(255) |
| uptime | INT |
| os_release | text |
| memory | json |

### scripts

| id | varchar(36) | uuid
| name | text |
| version | varchar(5) |
| output_regex | text |
| labels | json |
| script_content | text |

### hosts

| id | varchar(36) | uuid
| alias | text |
| attributes | json |
| last_pong | bool

### executions

| id | varchar(36) | uuid
| timestamp | TEXT | as ISO8601 strings ("YYYY-MM-DD HH:MM:SS.SSS")
| host_id | varchar(36) | uuid
| script_id | varchar(36) | uuid
| output | text |

### scheduling

| id | varchar(36) | uuid
| script_id | varchar(36) | uuid
<!-- run on host with attribute (label) xxx -->
| attributes | json |
| cron | text |

### metrics

| id | varchar(36) | uuid
| host_id | varchar(36) | uuid
| name | text |
| dimensions | json
| value | double |
