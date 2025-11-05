# jhttp

`jhttp` is a powerful and flexible command-line interface (CLI) tool built with Rust for defining, executing, and testing HTTP requests from a simple JSON file. It's designed to streamline API interaction, making it ideal for developers, testers, and anyone needing to automate HTTP calls.

## Why jhttp?

In a world of complex APIs and microservices, `jhttp` simplifies your workflow by:

- **Centralizing Request Definitions**: Keep all your API requests organized in a single, human-readable JSON file.
- **Automating API Testing**: Easily run a suite of requests to test your endpoints and get immediate feedback.
- **Environment Agnostic**: Seamlessly switch between development, staging, and production environments using environment variables.
- **Developer Friendly**: Built with Rust for performance and reliability, offering clear, color-coded output for quick analysis.

## Features

- **JSON-based Definitions**: Define a series of HTTP requests in a single, easy-to-read JSON file.
- **Multiple HTTP Methods**: Supports `GET`, `POST`, `PUT`, `DELETE`, and `PATCH`.
- **Customizable Requests**: Set custom headers, query parameters, JSON bodies, and form data.
- **Environment Variable Substitution**: Use `{{VARIABLE_NAME}}` syntax in your JSON file to substitute values from environment variables or a `.env` file.
- **Configurable Timeout**: Set a global timeout for all requests.
- **Flexible Output**: Choose between a human-readable, color-coded "pretty" format (default) or a structured `json` output for easy parsing and integration with other tools.
- **Test Summaries**: Get a quick overview of test results, including total, success, and failure counts, along with a success rate.

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) and Cargo (version 1.70 or later recommended).

## Installation

There are two primary ways to install and run `jhttp`:

### Option 1: Build from Source (for Development)

This method is recommended if you plan to contribute to `jhttp` or need the latest unreleased features.

1.  **Clone the repository:**
    ```sh
    git clone git@github.com:rlaehdals/jhttp.git # Replace with actual repository URL
    cd jhttp
    ```

2.  **Build the project:**
    ```sh
    cargo build --release
    ```
    The executable will be generated at `target/release/jhttp`.

### Option 2: Install with `cargo install` (for Users)

This is the easiest way to get `jhttp` up and running as a standalone command-line tool.

```sh
cargo install --path .
```

After installation, `jhttp` will be available in your Cargo binary path (`~/.cargo/bin` on Linux/macOS, or `%USERPROFILE%\.cargo\bin` on Windows), allowing you to run it from any directory.

## Usage

To run `jhttp`, you need to provide a JSON file containing your request definitions.

If you built from source (Option 1), execute it directly:
```sh
./target/release/jhttp --file request.json
```

If you installed with `cargo install` (Option 2), you can run `jhttp` directly:
```sh
jhttp --file request.json
```

### Command-Line Arguments

-   `--file <PATH>` or `-f <PATH>`: (Required) Path to the JSON file containing request definitions.
-   `--timeout <SECONDS>` or `-t <SECONDS>`: (Optional) Request timeout in seconds. Defaults to `30`.
-   `--output <FORMAT>` or `-o <FORMAT>`: (Optional) Output format. Available options are `pretty` (default) and `json`.

## JSON Request Format

The core of `jhttp` is the JSON file that defines the requests. It should be an array of request objects.

Each request object can have the following fields:

-   `name` (string, optional): A descriptive name for the request. This name is used in the output summary.
-   `url` (string, required): The target URL for the HTTP request.
-   `method` (string, required): The HTTP method to use (e.g., `"GET"`, `"POST"`, `"PUT"`, `"DELETE"`, `"PATCH"`).
-   `headers` (object, optional): A dictionary of key-value pairs for request headers (e.g., `{"Content-Type": "application/json"}`).
-   `params` (object, optional): A dictionary of key-value pairs for URL query parameters (e.g., `{"page": "1", "limit": "10"}`).
-   `body` (JSON object/array, optional): A JSON payload for methods like POST, PUT, PATCH. Cannot be used with `form`.
-   `form` (object, optional): A dictionary of key-value pairs for `application/x-www-form-urlencoded` data. Cannot be used with `body`.

### Example `request.json`

```json
[
  {
    "name": "1. Get a single post",
    "url": "https://jsonplaceholder.typicode.com/posts/1",
    "method": "GET"
  },
  {
    "name": "2. Create a new post with JSON body",
    "url": "https://jsonplaceholder.typicode.com/posts",
    "method": "POST",
    "headers": {
      "Content-Type": "application/json"
    },
    "body": {
      "title": "New Post",
      "body": "This is the content.",
      "userId": 1
    }
  },
  {
    "name": "3. Post form data",
    "url": "https://httpbin.org/post",
    "method": "POST",
    "form": {
      "username": "testuser",
      "status": "active"
    }
  }
]
```

## Environment Variables

`jhttp` supports dynamic value substitution using environment variables. This is particularly useful for managing sensitive information (like API keys) or configuring requests for different environments without modifying the JSON request file.

1.  **Define Environment Variables**: You can define environment variables directly in your shell or by creating a `.env` file in the directory where you run `jhttp`.

    **Example `.env` file:**
    ```
    AUTH_TOKEN="your-secret-token-123"
    API_HOST="api.dev.example.com"
    ```

2.  **Reference in JSON**: Use the `{{VARIABLE_NAME}}` syntax within your `request.json` file.

### Example with Environment Variables

**`.env` file:**
```
TEST_HOST="httpbin.org"
AUTH_TOKEN="fake-token-12345"
```

**`request.json`:**
```json
[
  {
    "name": "Get request with environment variables",
    "url": "https://{{TEST_HOST}}/get",
    "method": "GET",
    "headers": {
      "Authorization": "Bearer {{AUTH_TOKEN}}"
    }
  }
]
```

`jhttp` will automatically substitute `{{TEST_HOST}}` with `httpbin.org` and `{{AUTH_TOKEN}}` with `fake-token-12345` before sending the request.

## Output Formats

`jhttp` provides two distinct output formats to suit different needs:

### Pretty (Default)

The default output is designed for human readability. It's color-coded and provides a clear, step-by-step breakdown of each request's execution and its response.

```text
============================================================
HTTP Request Test Started (Timeout: 30s)
============================================================

[1/2] Get a single post
Method: GET https://jsonplaceholder.typicode.com/posts/1
✅ Status: 200 OK
Response time: 0.25s

Response body:
{
  "userId": 1,
  "id": 1,
  "title": "sunt aut facere repellat provident occaecati excepturi optio reprehenderit",
  "body": "quia et suscipit\nsuscipit recusandae consequuntur expedita et cum\nreprehenderit molestiae ut ut quas totam\nnostrum rerum est autem sunt rem eveniet architecto"
}
------------------------------------------------------------

[2/2] Create a new post with JSON body
Method: POST https://jsonplaceholder.typicode.com/posts
✅ Status: 201 Created
Response time: 0.15s

Response body:
{
  "title": "New Post",
  "body": "This is the content.",
  "userId": 1,
  "id": 101
}
------------------------------------------------------------

┌───────────────────────┐
│      Test Summary     │
├───────────────────────┤
│  Total: 2             │
│  Success: 2           │
│  Failed: 0            │
│  Success rate: 100.0% │
└───────────────────────┘
```

### JSON

The `json` output format prints a structured JSON summary to standard output. This format is ideal for scripting, automation, and integration with other tools that can parse JSON.

To run with JSON output:

```sh
./target/release/jhttp -f request.json -o json
```

To save the JSON output directly to a file (e.g., `results.json`), use your shell's output redirection:

```sh
./target/release/jhttp -f request.json -o json > results.json
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details. (Note: A `LICENSE` file should be created in the project root if not already present.)
