## Testing on an Umbrel device.


If you have an Umbrel device and want to test this app on it, do the following:

1. Go to the root folder of your Umbrel system (where the "apps", "app-data", "lnd", etc. folders are) via SSH. Perhaps `cd ~/umbrel`?
2. Create a new subfolder within the "apps" folder called "podcasting20-boosts": `mkdir apps/podcasting20-boosts`
3. Create a "docker-compose.yml" file in that new folder: `pico apps/podcasting20-boosts/docker-compose.yml`
4. Paste into that file, the contents found [here](docker-compose.yml) and save the file.
5. You should still be in the root folder of your Umbrel system.  Run this command: `sudo scripts/app install podcasting20-boosts`.

If the results of running that command look good, you should now be able to get to the boost-a-gram app by going to `http://<yourumbrel>:2112`
in your browser.

It can take a few minutes to populate any existing boostagrams you have received since it has to build the invoice database in the background
when it first starts up.

The app will not show up in your Umbrel dashboard when installed this way.  This is only for testing.

If you want to remove the app just run this command: `sudo scripts/app uninstall podcasting20-boosts`
