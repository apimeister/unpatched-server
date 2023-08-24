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
| last_pong | TEXT | as ISO8601 string ("YYYY-MM-DD HH:MM:SS")

### executions

| Name | Type | Comment
:--- | :--- | :---
| id | TEXT | uuid v4 hyphenated
| request | TEXT | as ISO8601 string ("YYYY-MM-DD HH:MM:SS")
| response | TEXT | as ISO8601 string ("YYYY-MM-DD HH:MM:SS")
| host_id | TEXT | uuid v4 hyphenated
| sched_id | TEXT | uuid v4 hyphenated
| created | TEXT | as ISO8601 string ("YYYY-MM-DD HH:MM:SS")
| output | TEXT | script output

#### Constraints

`FOREIGN KEY(host_id) REFERENCES hosts(id) ON DELETE CASCADE`  
`FOREIGN KEY(sched_id) REFERENCES schedules(id) ON DELETE CASCADE`

### schedules

| Name | Type | Comment
:--- | :--- | :---
| id | TEXT | uuid v4 hyphenated
| script_id | TEXT | uuid v4 hyphenated
| target_attributes | TEXT | server label to execute on
| target_host_id | TEXT | server uuid to execute on
| timer_cron | TEXT | cron pattern for execution
| timer_ts | TEXT | timestamp for execution
| active | NUMERIC | bool

#### Constraints

`FOREIGN KEY(script_id) REFERENCES scripts(id) ON DELETE CASCADE`  
`FOREIGN KEY(target_host_id) REFERENCES hosts(id) ON DELETE CASCADE`

### metrics - not implemented yet

| Name | Type | Comment
:--- | :--- | :---
| id | TEXT | uuid v4 hyphenated
| host_id | TEXT | uuid v4 hyphenated
| name | TEXT |
| dimensions | json
| value | double |
