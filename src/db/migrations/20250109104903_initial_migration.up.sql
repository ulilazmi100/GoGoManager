CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE users (
    user_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    email VARCHAR NOT NULL UNIQUE,
    password VARCHAR NOT NULL,
    name VARCHAR,
    user_image_uri VARCHAR,
    company_name VARCHAR,
    company_image_uri VARCHAR,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE departments (
    department_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_departments_name ON departments(name);

CREATE TABLE employees (
    employee_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    identity_number VARCHAR NOT NULL UNIQUE,
    name VARCHAR NOT NULL,
    employee_image_uri VARCHAR,
    gender VARCHAR NOT NULL,
    department_id UUID NOT NULL REFERENCES departments(department_id),
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_employees_identity_number ON employees(identity_number);
CREATE INDEX idx_employees_department_id ON employees(department_id);

CREATE TABLE files (
    file_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(user_id),
    uri VARCHAR NOT NULL,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_files_user_id ON files(user_id);