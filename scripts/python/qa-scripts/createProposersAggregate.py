from gspreadWrapper import GspreadWrapper
from options import Options
from utils import Utils

from gspread.models import Cell
from gspread_formatting import *

from time import sleep
import pandas as pd
import os

class createProposersAggregate():
    def __init__(self):
        self.opt = Options()
        self.utils = Utils()
        self.gspreadWrapper = GspreadWrapper()
        self.proposersFiles = []

        self.allColumns = [self.opt.notValidCol]

    def prepareBaseData(self):
        self.gspreadWrapper.getProposersMasterData()
        self.dfMasterProposers = self.gspreadWrapper.dfMasterProposers.set_index(self.opt.assessmentsIdCol)
        self.dfMasterProposers[self.opt.proposerMarkCol] = 0
        self.dfMasterProposers[self.opt.proposersRationaleCol] = ''

    def prepareProposersFileList(self):
        for currentDirPath, currentSubdirs, currentFiles in os.walk('./proposers-files'):
            for aFile in currentFiles:
                if aFile.endswith(".csv") :
                    fpath = str(os.path.join(currentDirPath, aFile))
                    self.proposersFiles.append(fpath)

    def loadProposersFiles(self):
        self.prepareBaseData()
        self.prepareProposersFileList()
        self.proposersData = []
        for proposerFile in self.proposersFiles:
            print("Loading {}".format(proposerFile))
            data = pd.read_csv(proposerFile)
            data.set_index(self.opt.assessmentsIdCol, inplace=True)
            data.fillna('', inplace=True)
            self.proposersData.append(data)

    def createDoc(self):
        self.loadProposersFiles()
        # Loop over master ids as reference
        for id, row in self.dfMasterProposers.iterrows():
            # Loop over all vca files
            for filesIdx, proposerDf in enumerate(self.proposersData):
                if (id in proposerDf.index):
                    locAss = proposerDf.loc[id]
                    if self.utils.checkIntegrity(id, row, locAss):
                        if (self.isProposerFeedbackValid(locAss)):
                            colVal = self.utils.checkIfMarked(locAss, self.opt.notValidCol)
                            if (colVal > 0):
                                self.dfMasterProposers.loc[id, self.opt.proposerMarkCol] = self.dfMasterProposers.loc[id, self.opt.proposerMarkCol] + colVal
                            ratioColVal = self.utils.checkIfMarked(
                                locAss, self.opt.notValidRationaleCol
                            )
                            if (ratioColVal > 0):
                                self.dfMasterProposers.loc[id, self.opt.proposersRationaleCol] = locAss[self.opt.notValidRationaleCol]
                    else:
                        fn = self.proposersFiles[filesIdx]
                        print("{} failed to pass the integrity test at id {}".format(fn, id))


        self.dfMasterProposers[self.opt.assessmentsIdCol] = self.dfMasterProposers.index
        self.dfMasterProposers.to_csv('cache/test-proposers-aggregate.csv')
        
        spreadsheet = self.gspreadWrapper.createDoc(self.opt.proposersAggregateFileName)

        # Print valid assessments
        assessmentsHeadings = [
            self.opt.assessmentsIdCol, self.opt.challengeCol,
            self.opt.proposalKeyCol, self.opt.ideaURLCol, self.opt.assessorCol,
            self.opt.tripletIdCol, self.opt.proposalIdCol,
            self.opt.q0Col, self.opt.q0Rating, self.opt.q1Col, self.opt.q1Rating,
            self.opt.q2Col, self.opt.q2Rating, self.opt.blankCol,
            self.opt.proposerMarkCol, self.opt.proposersRationaleCol
        ]
        assessmentsWidths = [
            ('A', 30), ('B:D', 150), ('E', 100), ('F:G', 40), ('H', 300),
            ('I', 30), ('J', 300), ('K', 30), ('L', 300), ('M:0', 30),
            ('P', 300)
        ]
        assessmentsFormats = [
            ('A', self.utils.counterFormat),
            ('I', self.utils.counterFormat),
            ('K', self.utils.counterFormat),
            ('M', self.utils.counterFormat),
            ('N', self.utils.counterFormat),
            ('A1:P1', self.utils.headingFormat),
            ('N1:O1', self.utils.verticalHeadingFormat),
            ('I1', self.utils.verticalHeadingFormat),
            ('K1', self.utils.verticalHeadingFormat),
            ('M1', self.utils.verticalHeadingFormat),
            ('H2:H', self.utils.textFormat),
            ('J2:J', self.utils.textFormat),
            ('L2:L', self.utils.textFormat),
        ]

        self.gspreadWrapper.createSheetFromDf(
            spreadsheet,
            'Assessments',
            self.dfMasterProposers,
            assessmentsHeadings,
            columnWidths=assessmentsWidths,
            formats=assessmentsFormats
        )

        print('Link: {}'.format(spreadsheet.url))

    def isFilteredOutValid(self, row):
        return (
            not self.utils.checkIfMarked(row, self.opt.notValidCol) or
            self.utils.checkIfMarked(row, self.opt.notValidRationaleCol)
        )

    def isProposerFeedbackValid(self, row):
        return self.isFilteredOutValid(row)

c = createProposersAggregate()
c.createDoc()
