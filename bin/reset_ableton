# select from installed versions of ableton
# list installed versions of ableton
ls $HOME/Library/Preferences/Ableton/

# get the version number
read -p "Enter the version number to reset: " VERSION

# Once user selects
cp $HOME/Library/Preferences/Ableton/$VERSION/Preferences.cfg $HOME/Desktop/Preferences.backup.cfg

# remove the preferences file
rm $HOME/Library/Preferences/Ableton/$VERSION/Preferences.cfg

# rm -rf Undo folder
rm -rf $HOME/Library/Preferences/Ableton/$VERSION/Undo

# go to the template set 
ls $HOME/Music/Ableton/User\ Library/Templates

# copy these to the desktop into a folder called 'Templates Backups'
cp -r $HOME/Music/Ableton/User\ Library/Templates $HOME/Desktop/Templates\ Backups
# remove the Templates folder

rm -rf $HOME/Music/Ableton/User\ Library/Templates