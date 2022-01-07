import json

class Options():
    def __init__(self):
        self.loadOptions()

    def loadOptions(self):
        try:
            with open('options.json', 'r') as f:
                options = json.load(f)
                for k in options:
                    self.setOption(k, options[k])
        except:
            print("Error loading options.json")

    def setOption(self, target, value):
        setattr(self, target, value)
