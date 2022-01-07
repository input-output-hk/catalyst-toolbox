from gspreadWrapper import GspreadWrapper
from options import Options
from utils import Utils

from gspread_formatting import *
import pandas as pd
import json

class CreateProposerDocument():
    def __init__(self):
        self.opt = Options()
        self.utils = Utils()
        self.gspreadWrapper = GspreadWrapper()

    def filterOutCAProposers(self, proposersDf):
        toInclude = []
        toExclude = []
        proposals = json.load(open('proposals.json'))
        users = json.load(open('users.json'))
        for id, row in proposersDf.iterrows():
            # Get the user and proposal for each assessment from the json files.
            ass = row.to_dict()
            user = next((item for item in users if item["id"] == ass[self.opt.assessorCol]), None)
            proposal = next((item for item in proposals if item["id"] == ass[self.opt.proposalIdCol]), None)
            if (user and proposal):
                if (proposal["category"] in user["campaigns"]):
                    # Exclude the assessment if the assessment proposal category
                    # is in the user "campaigns" => challenges
                    toExclude.append(ass)
                else:
                    toInclude.append(ass)
            else:
                toInclude.append(ass)
        return pd.DataFrame(toInclude), pd.DataFrame(toExclude)


    def createDoc(self):
        pd.options.display.max_columns = 100
        print('Loading original...')
        self.gspreadWrapper.loadAssessmentsFile()
        proposerDf = self.gspreadWrapper.prepareDataFromExport()
        spreadsheet = self.gspreadWrapper.createDoc(
            self.opt.proposerDocumentName
        )
        # Define all the columns needed in the file
        headings = [
            self.opt.assessmentsIdCol, self.opt.challengeCol,
            self.opt.proposalKeyCol, self.opt.ideaURLCol, self.opt.assessorCol,
            self.opt.tripletIdCol, self.opt.proposalIdCol,
            self.opt.q0Col, self.opt.q0Rating, self.opt.q1Col, self.opt.q1Rating,
            self.opt.q2Col, self.opt.q2Rating, self.opt.blankCol,
            self.opt.notValidCol, self.opt.notValidRationaleCol
        ]

        print('Assign blanks...')
        # Assign 'x' for blank assessments
        proposerDf[self.opt.blankCol] = proposerDf.apply(
            lambda r: 'x' if (
                (str(r[self.opt.q0Col]).strip() == "") or
                (str(r[self.opt.q1Col]).strip() == "") or
                (str(r[self.opt.q2Col]).strip() == "") or
                (str(r[self.opt.q0Rating]).strip() == "NA") or
                (str(r[self.opt.q1Rating]).strip() == "NA") or
                (str(r[self.opt.q2Rating]).strip() == "NA")
            ) else ''
        , axis=1)

        toInclude, toExclude = self.filterOutCAProposers(proposerDf)

        print('Format columns...')
        widths = [
            ('A', 30), ('B', 100), ('C:D', 150), ('E', 100), ('F:G', 50),
            ('H', 300), ('I', 30), ('J', 300), ('K', 30), ('L', 300),
            ('M:O', 30), ('P', 250)
        ]

        formats = [
            ('A', self.utils.counterFormat),
            ('I', self.utils.counterFormat),
            ('K', self.utils.counterFormat),
            ('M', self.utils.counterFormat),
            ('N', self.utils.counterFormat),
            ('A1:P1', self.utils.headingFormat),
            ('F1:G1', self.utils.verticalHeadingFormat),
            ('N1:O1', self.utils.verticalHeadingFormat),
            ('I1', self.utils.verticalHeadingFormat),
            ('K1', self.utils.verticalHeadingFormat),
            ('M1', self.utils.verticalHeadingFormat),
            ('O2:O', self.utils.yellowFormat),
            ('H2:H', self.utils.textFormat),
            ('J2:J', self.utils.textFormat),
            ('L2:L', self.utils.textFormat),
        ]

        self.gspreadWrapper.createSheetFromDf(
            spreadsheet,
            "Assessments",
            toInclude,
            headings,
            widths,
            formats
        )

        if (len(toExclude)):
            self.gspreadWrapper.createSheetFromDf(
                spreadsheet,
                "Assessments Excluded (CA proposer in same challenge)",
                toExclude,
                headings,
                widths,
                formats
            )
        print('Master Document for proposers created')
        print('Link: {}'.format(spreadsheet.url))

c = CreateProposerDocument()
c.createDoc()
