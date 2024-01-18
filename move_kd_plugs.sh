#!/bin/bash

sudo cp -r *.vst /Library/Audio/Plug-Ins/VST/
sudo cp -r *.vst3 /Library/Audio/Plug-Ins/VST3/
sudo cp -r *.component /Library/Audio/Plug-Ins/Components/

echo "All k'd plugins moved successfully."
