# GoGoManager
GoGoManager is an app for helping managers manage their employees, ProjectSprint Batch 3 Week 1 project.

## Setup

1. Clone the repository.
2. Create a `.env` file and set the required environment variables.
3. Run the database migrations:
   ```bash
   sqlx migrate run
   ```
4. Build and run the application:
   ```bash
   cargo run
   ```

## API Endpoints

- `POST /v1/auth`: User authentication.
- `GET /v1/user`: Retrieve user profile.
- `PATCH /v1/user`: Update user profile.
- `POST /v1/file`: Upload a file.
- `POST /v1/employee`: Create a new employee.
- `GET /v1/employee`: Retrieve employees.
- `PATCH /v1/employee/:identityNumber`: Update an employee.
- `DELETE /v1/employee/:identityNumber`: Delete an employee.
- `POST /v1/department`: Create a new department.
- `GET /v1/department`: Retrieve departments.
- `PATCH /v1/department/:departmentId`: Update a department.
- `DELETE /v1/department/:departmentId`: Delete a department.

## Environment Variables

- `DATABASE_URL`: The connection string for the PostgreSQL database.
- `JWT_SECRET`: The secret key used for JWT token generation.
- `AWS_ACCESS_KEY_ID`: The AWS access key ID for S3 integration.
- `AWS_SECRET_ACCESS_KEY`: The AWS secret access key for S3 integration.
- `AWS_REGION`: The AWS region for S3 integration.
- `AWS_S3_BUCKET`: The S3 bucket name for file uploads.