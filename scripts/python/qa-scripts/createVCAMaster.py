from gspreadWrapper import GspreadWrapper
from options import Options
from utils import Utils

from gspread.models import Cell
from gspread_formatting import *

class CreateVCAMaster():
    def __init__(self):
        self.opt = Options()
        self.utils = Utils()
        self.gspreadWrapper = GspreadWrapper()

    def createDoc(self):
        spreadsheet = self.gspreadWrapper.createDoc(self.opt.VCAMasterFileName)

        # Define headings for VCAMasterFile
        print('Define headings...')
        headings = [
            self.opt.assessmentsIdCol, self.opt.challengeCol,
            self.opt.proposalKeyCol, self.opt.ideaURLCol, self.opt.assessorCol,
            self.opt.tripletIdCol, self.opt.proposalIdCol,
            self.opt.q0Col, self.opt.q0Rating, self.opt.q1Col, self.opt.q1Rating,
            self.opt.q2Col, self.opt.q2Rating, self.opt.proposerMarkCol,
            self.opt.proposersRationaleCol, self.opt.excellentCol,
            self.opt.goodCol, self.opt.notValidCol, self.opt.vcaFeedbackCol
        ]

        print('Load proposers flagged reviews...')
        self.gspreadWrapper.getProposersAggregatedData()

        # Extract assessors
        assessors = self.gspreadWrapper.dfProposers.groupby(
            self.opt.assessorCol
        ).agg(
            total=(self.opt.tripletIdCol, 'count'),
            blanks=(self.opt.blankCol, (lambda x: (x == 'x').sum()))
        )

        # Calculate and extract assessors by blanks
        assessors['blankPercentage'] = assessors['blanks'] / assessors['total']
        assessors['excluded'] = (assessors['blankPercentage'] >= self.opt.allowedBlankPerAssessor)
        excludedAssessors = assessors[(assessors['excluded'] == True)].index.tolist()
        includedAssessors = assessors[(assessors['excluded'] != True)].index.tolist()

        assessors['assessor'] = assessors.index

        # Filter out assessments made by excluded assessors
        validAssessments = self.gspreadWrapper.dfProposers[
            self.gspreadWrapper.dfProposers[self.opt.assessorCol].isin(includedAssessors)
        ]

        # Filter out blank assessments
        validAssessments = validAssessments[validAssessments[self.opt.blankCol] != 'x']

        # Remove proposers marks
        criteria = [self.opt.excellentCol, self.opt.goodCol, self.opt.notValidCol]
        for col in criteria:
            validAssessments[col] = ''

        # Assign 'x' for marks
        validAssessments[self.opt.proposerMarkCol] = validAssessments[self.opt.proposerMarkCol].apply(
            lambda r: 'x' if (r > 0) else ''
        )

        # Write sheet with assessments
        assessmentsWidths = [
            ('A', 30), ('B:D', 150), ('E', 100), ('F:G', 40), ('H', 300),
            ('I', 30), ('J', 300), ('K', 30), ('L', 300), ('M:N', 30),
            ('O', 300), ('P:R', 30), ('S', 300)
        ]
        assessmentsFormats = [
            ('A', self.utils.counterFormat),
            ('I', self.utils.counterFormat),
            ('K', self.utils.counterFormat),
            ('M', self.utils.counterFormat),
            ('N', self.utils.counterFormat),
            ('A1:S1', self.utils.headingFormat),
            ('I1', self.utils.verticalHeadingFormat),
            ('K1', self.utils.verticalHeadingFormat),
            ('M1', self.utils.verticalHeadingFormat),
            ('N1', self.utils.verticalHeadingFormat),
            ('P1:R1', self.utils.verticalHeadingFormat),
            ('H2:H', self.utils.textFormat),
            ('J2:J', self.utils.textFormat),
            ('L2:L', self.utils.textFormat),
            ('P2:P', self.utils.greenFormat),
            ('Q2:Q', self.utils.greenFormat),
            ('R2:R', self.utils.yellowFormat),
            ('S2:S', self.utils.textFormat),
        ]

        self.gspreadWrapper.createSheetFromDf(
            spreadsheet,
            'Assessments',
            validAssessments,
            headings,
            columnWidths=assessmentsWidths,
            formats=assessmentsFormats
        )

        # Write sheet with CAs summary
        self.gspreadWrapper.createSheetFromDf(
            spreadsheet,
            'Community Advisors',
            assessors,
            ['assessor', 'total', 'blanks', 'blankPercentage', 'excluded'],
            columnWidths=[('A', 140), ('B:D', 60), ('E', 100)],
            formats=[
                ('B:C', self.utils.counterFormat),
                ('D2:D', self.utils.percentageFormat),
                ('A1:E1', self.utils.headingFormat),
                ('B1:D1', self.utils.verticalHeadingFormat),
            ]
        )
        print('Master Document for vCAs created')
        print('Link: {}'.format(spreadsheet.url))

cvca = CreateVCAMaster()
cvca.createDoc()
