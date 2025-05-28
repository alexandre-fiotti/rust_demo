# GitHub Star Tracker

A Rust-based service that tracks and analyzes GitHub repository star metrics over time. This project provides a robust API to fetch, store, and analyze star data for any GitHub repository.

## Features

- 🌟 Track GitHub repository stars with timestamps
- 📊 Analyze daily star count trends
- 🚀 High-performance Rust implementation
- 🗄️ PostgreSQL database for reliable data storage
- 🔄 Automatic pagination handling for large repositories
- 🔒 Secure GitHub token-based authentication

## Prerequisites

- Rust (latest stable version)
- Docker and Docker Compose
- A GitHub Personal Access Token with repo scope

## Project Structure

```md
.
├── projects/
│   └── databases/          # Main database service
├── interfaces/
│   └── github/
│       └── stargazers/    # GitHub API integration
├── utils/
│   └── trace/             # Logging and tracing utilities
└── scripts/
    └── database/          # Database management scripts
```

## Setup

1. Clone the repository:

   ```sh
   git clone https://github.com/alexandre-fiotti/rust_demo.git
   cd rust_demo
   ```

2. Create a `.env` file in the `projects/databases` directory with the following content:

   ```md
   DATABASE_URL=postgresql://rustuser:password@localhost:5432/stargazer
   GITHUB_TOKEN=your_github_token_here
   ```

3. Start the PostgreSQL database:

   ```sh
   ./scripts/database/run_postgres.sh
   ```

## Database Management

### Starting and Stopping

```sh
# Start PostgreSQL container
./scripts/database/run_postgres.sh

# Stop PostgreSQL container
./scripts/database/stop_postgres.sh
```

### Database Connection

Connect to the database for manual inspection:

```sh
docker exec -it postgres_db psql -U rustuser -d stargazer
```

### Common PostgreSQL Commands

```sql
\dt        -- List all tables
\d TABLE   -- Describe table
\q         -- Exit psql
```

### Database Migrations

This project uses [Diesel](https://diesel.rs/) for database management. First, install the Diesel CLI:

```sh
cargo install diesel_cli --no-default-features --features postgres
```

Navigate to the databases project:

```sh
cd projects/databases
```

Run the migrations:

```sh
diesel migration run
```

Other useful Diesel commands:

```sh
diesel migration revert    # Undo the last migration
diesel migration redo     # Redo the last migration
diesel migration list    # List all migrations
diesel print-schema     # Print the current schema.rs
```

### Development Database Reset

If you need to completely reset the database during development:

1. First, clean the database:

   ```sql
   DROP SCHEMA public CASCADE;
   CREATE SCHEMA public;
   ```

2. Then, rerun all migrations:

   ```sh
   cd projects/databases
   diesel migration run
   ```

This will give you a clean slate with the proper schema for development.

## API Endpoints

### Update Repository Stars

```http
POST /github/repo_stars/update
Content-Type: application/json

{
    "owner": "repository_owner",
    "name": "repository_name"
}
```

### Get Daily Star Count

```http
POST /github/repo_stars/read_per_day
Content-Type: application/json

{
    "owner": "repository_owner",
    "name": "repository_name"
}
```

## Development

1. Build the project:

   ```sh
   cargo build
   ```

2. Run the service:

   ```sh
   cargo run -p projects_databases
   ```

The service will start on `http://0.0.0.0:8000`.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Author

Alexandre Fiotti
