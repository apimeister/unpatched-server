# DB Tables

## table structures

### scripts

| id | varchar(36) | uuid
| name | text |
| version | varchar(5) |
| output_regex | text |
| labels | json |
| timeout | text
| script_content | text |

### hosts

| id | varchar(36) | uuid
| alias | text |
| attributes | json |
| last_pong | bool

### executions

| id | TEXT | uuid
| request | TEXT | as ISO8601 string ("YYYY-MM-DD HH:MM:SS.SSS")
| response | TEXT | as ISO8601 string ("YYYY-MM-DD HH:MM:SS.SSS")
| host_id | TEXT | uuid
| script_id | TEXT | uuid
| output | TEXT |

### schedules

| id | varchar(36) | uuid
| script_id | varchar(36) | uuid
| attributes | json |
| cron | text |
| active | bool |

### metrics

| id | varchar(36) | uuid
| host_id | varchar(36) | uuid
| name | text |
| dimensions | json
| value | double |
