# DB Tables

## table structures

### scripts

| Name | Type | Comment
:--- | :--- | :---
| id | TEXT | uuid v4 hyphenated
| name | TEXT |
| version | TEXT |
| output_regex | TEXT |
| labels | TEXT | json |
| timeout | TEXT |
| script_content | TEXT |

### hosts

| Name | Type | Comment
:--- | :--- | :---
| id | TEXT | uuid v4 hyphenated
| alias | TEXT |
| attributes | TEXT | json |
| last_pong | TEXT | as ISO8601 string ("YYYY-MM-DD HH:MM:SS.SSS")

### executions

| Name | Type | Comment
:--- | :--- | :---
| id | TEXT | uuid v4 hyphenated
| request | TEXT | as ISO8601 string ("YYYY-MM-DD HH:MM:SS.SSS")
| response | TEXT | as ISO8601 string ("YYYY-MM-DD HH:MM:SS.SSS")
| host_id | TEXT | uuid v4 hyphenated
| script_id | TEXT | uuid v4 hyphenated
| sched_id | TEXT | uuid v4 hyphenated
| output | TEXT | script output

### schedules

| Name | Type | Comment
:--- | :--- | :---
| id | TEXT | uuid v4 hyphenated
| script_id | TEXT | uuid v4 hyphenated
| attributes | TEXT | json |
| cron | TEXT |
| active | NUMERIC | bool

### metrics - not implemented yet

| Name | Type | Comment
:--- | :--- | :---
| id | TEXT | uuid v4 hyphenated
| host_id | TEXT | uuid v4 hyphenated
| name | TEXT |
| dimensions | json
| value | double |
