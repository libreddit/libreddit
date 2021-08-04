# Unraid Docker Libreddit Installation

**I have added a Libreddit docker container template within Unraid. See the community apps section within your Unraid Server for easier installation.**
**The template version uses the same docker container developed and maintained by @spikecodes and is merely a template to get it easily installed**

This Unraid Docker Installation guide will mostly assume a few things;

1.	You have docker enabled within Unraid

2.	You have enabled community apps within Unraid

3.	You have enabled within settings the ability to utilize dockerhub for search results (_see settings within apps tab_)

4.	_OPTIONAL - You have a reverse proxy container and network to allow for certificate handling & SSL connections_

With that in mind, the installation of Libreddit is rather simple once you have the above setup.

5.	Head over to apps and search for Libreddit

6.	Click to begin the installation of Libreddit within the search result. (_The repo is spikecodes/libreddit_)

7.	Set the toggle on the right in the template as ‘advanced view’ (_It defaults to basic view_)

8.	Set your ‘Icon URL’ as  https://raw.githubusercontent.com/spikecodes/libreddit/master/static/logo.png (_This will provide you with the Libreddit logo_)

9.	Set your ‘WebUI’ as http://[IP]:[PORT:8080]/ (_This could be changed to whatever suits your local server port requirements - see below_)

10.	Set ‘Extra Parameters’ as --restart=always

11.	Set your network type as needed (_OPTIONAL - Set network type as your network that you utilize for your SSL certs (for me its proxnetwork)._)

12.	Add in your static IP address that you will utilize for Libreddit. (_It makes it easier to get to your hosted instance_)

13.	Now add in a ‘port’ as;
•	Name – Port
•	Container Port – 8080
•	Host Port – 8080 (_this could be changed to whatever suits your local server port requirements if your 8080 is already in use_)
•	Default Value – 8080 (_this could be changed to whatever suits your local server port requirements - as above_)
•	Connection Type – TCP
•	Description – Container Port: 8080

14.	Click apply to download/install/start the container.

15.	_OPTIONAL – Head over to your SSL cert provider container of choice and set-up as necessary to server certs to your Libreddit instance for your domain._
