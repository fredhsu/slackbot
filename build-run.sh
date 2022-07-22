docker build -t slackbot .
# Need to add ports that should be exposed if any
docker run -it --rm --name slackbot slackbot
