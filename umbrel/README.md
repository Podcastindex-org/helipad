## Testing on an Umbrel device.


If you have an Umbrel device running on the standard Raspberry Pi and want to test this app on it, do the following:

1. SSH into your Umbrel (which will require your Umbrel password):

        ssh umbrel@umbrel.local

2. While in the home directory, prompt will show `umbrel@umbrel:~`, clone this repo to your Umbrel:

        git clone https://github.com/Podcastindex-org/helipad.git

3. Change to the `umbrel` sub-directory of the helipad repo:

        cd helipad/umbrel

4. Run the install script with the root folder of your Umbrel install as the only argument:

        ./install.sh ~/umbrel/

or you may need this if you've done a non-standard Umbrel install `./install.sh /opt/umbrel`

If the results of running that command look good, you should now be able to get to the boost-a-gram app by going to `http://umbrel.local:2112`
in your browser. You might need to use your Umbrel's IP address if `umbrel.local` doesn't work for you.

It can take a few minutes to populate any existing boostagrams you have received since it has to build the invoice database in the background
when it first starts up.

The app will not show up in your Umbrel dashboard when installed this way.  This is only for testing.

If you want to remove the app just run this command: `sudo scripts/app uninstall podcasting20-boosts` from your Umbrel's root directory.
