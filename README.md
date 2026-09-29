# Support Ticketing API

A Rust backend for a support ticketing platform. Customers submit tickets, agents get notified by email and reply from a dashboard.

## Stack

- Language: Rust (Axum, Tokio)
- Database: Postgres via Supabase
- Auth: Supabase Auth (JWT)
- Email: Resend

## Features

- Customers submit tickets and view their own ticket history
- Agents see all tickets, filter by status/priority, search, and reply
- New tickets notify every agent by email
- Agent replies email the customer
- Internal notes, visible to agents only
- Row Level Security backs the API's own access checks

## Routes

| Method | Path                   | Description                                       |
| ------ | ---------------------- | ------------------------------------------------- |
| GET    | /health                | Health check                                      |
| GET    | /me                    | Current user profile                              |
| POST   | /tickets               | Create a ticket                                   |
| GET    | /tickets               | List tickets (filterable by status, priority, q)  |
| GET    | /tickets/{id}          | Get a ticket with its message thread              |
| PATCH  | /tickets/{id}          | Update status, priority, or assignee (agent only) |
| POST   | /tickets/{id}/messages | Reply to a ticket                                 |

## Setup

1. Create a Supabase project and run the migration in migrations/ against it
2. Copy .env.example to .env and fill in:
   - DATABASE_URL: Supabase Postgres connection string (session pooler recommended)
   - SUPABASE_JWT_SECRET: from Settings, API, JWT Secret (legacy)
   - RESEND_API_KEY: from resend.com
3. Fill in config.toml with support_inbox, from_email, and port
4. Give at least one signed up user the agent role in the profiles table so there's someone to receive ticket notifications
5. Run it:

cargo run

## Deployment

Builds as a Docker container. Deployed on Railway, environment variables are set in the Railway dashboard rather than committed.
