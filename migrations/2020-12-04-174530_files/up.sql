CREATE TABLE files (
  id serial primary key,
  record_id int not null constraint files_record_id references records,
  kind text not null,
  local_path text,
  remote_path text not null,
  remote_id text,
  file_name text
)