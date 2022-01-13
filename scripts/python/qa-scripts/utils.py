from options import Options

import json
from gspread_formatting import *

class Utils():
    def __init__(self):
        self.opt = Options()
        # Global cells style
        self.counterFormat = cellFormat(
            textFormat=textFormat(bold=True, fontSize=10),
            horizontalAlignment='CENTER'
        )
        self.percentageFormat = cellFormat(
            numberFormat=numberFormat(type='PERCENT', pattern="##.###%"),
            horizontalAlignment='RIGHT'
        )
        self.noteFormat = cellFormat(
            wrapStrategy='CLIP',
            textFormat=textFormat(fontSize=10),
        )
        self.textFormat = cellFormat(
            textFormat=textFormat(fontSize=10),
        )
        self.headingFormat = cellFormat(
            backgroundColor=color(0.71, 0.85, 1),
            textFormat=textFormat(bold=True, fontSize=12),
            horizontalAlignment='CENTER'
        )
        self.verticalHeadingFormat = cellFormat(
            backgroundColor=color(0.71, 0.85, 1),
            textFormat=textFormat(bold=True, fontSize=12),
            textRotation=textRotation(angle=90),
            verticalAlignment='BOTTOM'
        )
        self.yellowFormat = cellFormat(
            backgroundColor=color(1, 0.94, 0.58),
            textFormat=textFormat(bold=True),
            horizontalAlignment='CENTER'
        )
        self.redFormat = cellFormat(
            backgroundColor=color(1, 0.58, 0.58),
            textFormat=textFormat(bold=True),
            horizontalAlignment='CENTER'
        )
        self.greenFormat = cellFormat(
            backgroundColor=color(0.73, 1, 0.70),
            textFormat=textFormat(bold=True),
            horizontalAlignment='CENTER'
        )

    '''
    saveCache() saves the pulled records in a json file to cache the response.
    '''
    def saveCache(self, dicts, name):
        print('Saving cache..')
        with open('cache/' + name + '.json', 'w') as f:
            json.dump(dicts, f)

    '''
    loadCache() get records from cache if present.
    '''
    def loadCache(self, name):
        try:
            with open('cache/' + name + '.json', 'r') as f:
                data = json.load(f)
            return data
        except:
            return False

    '''
    Used in proposers and vCAs aggregatation
    '''

    def checkIntegrity(self, id, ass1, ass2):
        return (
            (ass1[self.opt.proposalIdCol] == ass2[self.opt.proposalIdCol]) and
            (ass1[self.opt.q0Rating] == ass2[self.opt.q0Rating]) and
            (ass1[self.opt.q1Rating] == ass2[self.opt.q1Rating]) and
            (ass1[self.opt.q2Rating] == ass2[self.opt.q2Rating]) and
            (ass1[self.opt.assessorCol] == ass2[self.opt.assessorCol])
        )

    '''
    Used in proposers and vCAs aggregatation
    '''
    def checkIfMarked(self, row, column):
        return (row[column].strip() != '')
