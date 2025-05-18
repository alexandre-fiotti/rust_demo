# rust_demo

## Exploitation

### Running the database

```sh
./scripts/database/run_postgres.sh
./scripts/database/stop_postgres.sh
```

Then, to connect to the db to check it:

```sh
docker exec -it postgres_db psql -U rustuser -d stargazer
```

And then, check around:

```sql
\dt : check the existing tables
\q : exit
```

To completely wipe the db:

```sql
DROP SCHEMA public CASCADE;
CREATE SCHEMA public;
```
