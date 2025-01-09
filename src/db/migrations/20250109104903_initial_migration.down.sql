DROP INDEX IF EXISTS idx_files_user_id;
DROP TABLE IF EXISTS files;

DROP INDEX IF EXISTS idx_employees_department_id;
DROP INDEX IF EXISTS idx_employees_identity_number;
DROP TABLE IF EXISTS employees;

DROP INDEX IF EXISTS idx_departments_name;
DROP TABLE IF EXISTS departments;

DROP TABLE IF EXISTS users;

DROP EXTENSION IF EXISTS "uuid-ossp";
