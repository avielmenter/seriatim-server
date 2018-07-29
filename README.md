# Seriatim Server
The server for [Seriatim](https://github.com/avielmenter/seriatim).

# About
This repository contains the code for the back-end server for [Seriatim](https://github.com/avielmenter/seriatim), a web application for creating outlines. See the [Seriatim](https://github.com/avielmenter/seriatim) repository for more information about the application.

# Setup
If you want to download and run the Seriatim server yourself, you can do so by following these steps:

## Prerequisites
To run this project, you must be using the correct nightly version of the Rust compiler. The specific version required can be found in the [RustConfig](https://github.com/avielmenter/seriatim-server/blob/master/RustConfig) file.

Additionally, you must have an accessible [PostgreSQL databse](https://www.postgresql.org/), which Seriatim will use to store its data.

## Compilation
To compile the Seriatim server, perform the following steps:

 - Clone this repository using the command `git clone https://github.com/avielmenter/seriatim-server.git`.
 - Navigate to the `seriatim-server` folder.
 - Run the command `cargo build --release`.

## Environment Variables

To run this application, you must configure certain environment variables on your system:

 - `DATABASE_URL`: The URL (including login information) at which your PostgreSQL database can be accessed.
 - `SERIATIM_ALLOWED_ORIGIN`: The URL of the Seriatim front-end, without a trailing `'/'`. The server will only be able to service CORS requests coming from this domain.
 - `SERIATIM_DOMAIN`: The domain at which the Seriatim server can be accessed, including a trailing `'/'`.
 - `SERIATIM_SESSION_DOMAIN`: The domain for which Seriatim's session cookies will be set.
 - `SERIATIM_TWITTER_KEY`: The key for Seriatim's Twitter application. This is necessary for users to log in via Twitter.
 - `SERIATIM_TWITTER_SECRET`: The secret for Seriatim's Twitter application.
 - `SERIATIM_GOOGLE_ID`: The ID for Seriatim's Google application. This is necessary for users to log in via Google.
 - `SERIATIM_GOOGLE_SECRET`: The secret for Seriatim's Google application.
 - `SERIATIM_GOOGLE_API_KEY`: The API Key for Seriatim's Google application.
 - `SERIATIM_FB_ID`: The ID for Seriatim's Facebook application. This is necessary for users to log in via Facebook.
 - `SERIATIM_FB_SECRET`: The secret for Seriatim's Facebook application.

 You can set these environment variables using your operating system, or you can configure them in a `.env` file placed at the root of the `seriatim-server` directory.

 A `template.env` file is included with this project. You can rename this file to `.env` and configure the variables contained within the file to set up your environment. Because the Seriatim server uses [Rocket](https://github.com/SergioBenitez/Rocket), the `template.env` file additionally contains certain [Rocket](https://github.com/SergioBenitez/Rocket) environment variables that you may also want to configure.

 ## Run
 To run the application, navigate to the `seriatim-server` folder and run the command `cargo run`.

 ## Development Mode
 The above instructions will compile and run the Seriatim server in production mode. If you wish to instead use the Seriatim server in development mode, you can compile the server using the command `cargo build`. Additionally, you should set the `ROCKET_ENV` environment variable to `'development'` rather than `'production'`.

 # License
 This project is licensed under the [MIT License](https://github.com/avielmenter/seriatim-server/blob/master/LICENSE).