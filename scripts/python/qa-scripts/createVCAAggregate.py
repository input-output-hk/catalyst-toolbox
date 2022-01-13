from gspreadWrapper import GspreadWrapper
from options import Options
from utils import Utils

from gspread.models import Cell
from gspread_formatting import *

from time import sleep
import pandas as pd
import json
import os

class createVCAAggregate():
    def __init__(self):
        self.opt = Options()
        self.utils = Utils()
        self.gspreadWrapper = GspreadWrapper()
        self.proposals = json.load(open('proposals.json'))
        self.vcas = json.load(open('vcas.json'))
        self.vcasFiles = []

        self.badColumns = [self.opt.notValidCol]
        self.goodColumns = [self.opt.goodCol]
        self.excellentColumns = [self.opt.excellentCol]
        self.allColumns = self.badColumns + self.goodColumns + self.excellentColumns

        self.dChallenges = self.opt.distinctChallenges


    def prepareBaseData(self):
        self.gspreadWrapper.getVCAMasterData()
        self.vcaMerged = pd.DataFrame(columns=self.gspreadWrapper.dfVca.columns, data=None)
        self.vcaMerged[self.opt.vcaName] = ''
        self.dfVca = self.gspreadWrapper.dfVca.set_index(self.opt.assessmentsIdCol)
        self.gspreadWrapper.getProposersMasterData()
        self.dfMasterProposers = self.gspreadWrapper.dfMasterProposers.set_index(self.opt.assessmentsIdCol)
        # Set all counters to 0
        self.dfVca[self.opt.noVCAReviewsCol] = 0
        for col in self.allColumns:
            self.dfVca[col] = 0
            self.dfVca['Result ' + col] = 0

    def prepareVCAsFileList(self):
        for currentDirPath, currentSubdirs, currentFiles in os.walk('./vcas-files'):
            for aFile in currentFiles:
                if aFile.endswith(".csv") :
                    fpath = str(os.path.join(currentDirPath, aFile))
                    self.vcasFiles.append(fpath)

    def loadVCAsFiles(self):
        self.prepareBaseData()
        self.prepareVCAsFileList()
        self.vcasData = []
        self.vcasFileList = []
        for vcaFile in self.vcasFiles:
            print("Loading {}".format(vcaFile))
            data = pd.read_csv(vcaFile)
            data.set_index(self.opt.assessmentsIdCol, inplace=True)
            data.fillna('', inplace=True)
            if not set([self.opt.notValidCol]).issubset(data.columns):
                data[self.opt.notValidCol] = data[self.opt.notValidAlternativeCol]
                data.drop(self.opt.notValidAlternativeCol, axis=1, inplace=True)
            data = self.filterVCAConficts(data, vcaFile)
            if data is not False:
                self.vcasData.append(data)
                self.vcasFileList.append(vcaFile)

    def createDoc(self):
        self.loadVCAsFiles()
        # Loop over master ids as reference
        for id, row in self.dfVca.iterrows():
            proposerAss = self.dfMasterProposers.loc[id]
            # Loop over all vca files
            for filesIdx, vcaDf in enumerate(self.vcasData):
                if (id in vcaDf.index):
                    locAss = vcaDf.loc[id]
                    integrity = self.utils.checkIntegrity(id, row, locAss)
                    vca_fn = self.vcasFileList[filesIdx]
                    vca_filename = os.path.basename(vca_fn.replace('\\',os.sep))
                    single_vca = next((item for item in self.vcas if item['vca_file'] == vca_filename), None)
                    if (integrity is False):
                        print("{} failed to pass the integrity test at id {}".format(vca_fn, id))
                    if integrity:
                        bad = self.badFeedback(locAss)
                        good = self.goodFeedback(locAss)
                        excellent = self.excellentFeedback(locAss)
                        if (self.isVCAfeedbackValid(locAss, bad, good, excellent)):
                            if (bad or good or excellent):
                                reviews_num_col = "No. of Reviews"
                                if locAss[self.opt.challengeCol] in self.dChallenges:
                                    reviews_num_col = reviews_num_col + " " + locAss[self.opt.challengeCol]
                                if reviews_num_col in single_vca:
                                    single_vca[reviews_num_col] = single_vca[reviews_num_col] + 1
                                else:
                                    single_vca[reviews_num_col] = 1
                                self.dfVca.loc[id, self.opt.noVCAReviewsCol] = self.dfVca.loc[id, self.opt.noVCAReviewsCol] + 1
                                # Append the single review to the vcaMerged file
                                toBeMergedAssessment = locAss.copy()
                                toBeMergedAssessment['id'] = id
                                toBeMergedAssessment[self.opt.vcaName] = single_vca[self.opt.vcaName]
                                self.vcaMerged = self.vcaMerged.append(toBeMergedAssessment)

                            for col in self.allColumns:
                                colVal = self.utils.checkIfMarked(locAss, col)
                                if (colVal > 0):
                                    self.dfVca.loc[id, col] = self.dfVca.loc[id, col] + colVal

            (bad, good, excellent) = self.calculateOutcome(self.dfVca.loc[id])
            self.dfVca.loc[id, 'Result ' + self.opt.notValidCol] = bad
            self.dfVca.loc[id, 'Result ' + self.opt.goodCol] = good
            self.dfVca.loc[id, 'Result ' + self.opt.excellentCol] = excellent

        vcaAggregatedAssessments = pd.DataFrame(self.dfVca).copy()
        vcaAggregatedAssessments.fillna('', inplace=True)
        vcaAggregatedAssessments[self.opt.assessmentsIdCol] = vcaAggregatedAssessments.index

        aggregatedAssessments = pd.DataFrame(self.dfVca).copy()
        aggregatedAssessments.fillna('', inplace=True)
        aggregatedAssessments[self.opt.assessmentsIdCol] = aggregatedAssessments.index
        aggregatedAssessments[self.opt.excellentCol] = aggregatedAssessments['Result ' + self.opt.excellentCol]
        aggregatedAssessments[self.opt.goodCol] = aggregatedAssessments['Result ' + self.opt.goodCol]
        aggregatedAssessments[self.opt.notValidCol] = aggregatedAssessments['Result ' + self.opt.notValidCol]

        # Select valid assessments (no filtered out, no blank assessments)
        validAssessments = pd.DataFrame(self.dfVca).copy()
        validAssessments.fillna('', inplace=True)
        validAssessments[self.opt.assessmentsIdCol] = validAssessments.index
        validAssessments = validAssessments[(
            (~(validAssessments['Result ' + self.opt.notValidCol] == 'x'))
        )]
        validAssessments[self.opt.excellentCol] = validAssessments['Result ' + self.opt.excellentCol]
        validAssessments[self.opt.goodCol] = validAssessments['Result ' + self.opt.goodCol]
        validAssessments[self.opt.notValidCol] = validAssessments['Result ' + self.opt.notValidCol]

        # Create a structured list for
        # dChallenges add the filtered df.
        validNatives = []
        for native in self.dChallenges:
            nativeEl = {
                "title": native,
                "validAssessments": validAssessments[(validAssessments[self.opt.challengeCol] == native)]
            }
            validNatives.append(nativeEl)

        # Create a general valid df without valid from dChallenges
        generalValidAssessments = validAssessments[(~validAssessments[self.opt.challengeCol].isin(self.dChallenges))]

        blanksAssessments = pd.DataFrame(self.dfMasterProposers).copy()
        blanksAssessments.fillna('', inplace=True)
        blanksAssessments[self.opt.assessmentsIdCol] = blanksAssessments.index
        blanksAssessments = blanksAssessments[(
            (blanksAssessments[self.opt.blankCol] == 'x')
        )]
        blanksAssessments.drop(self.opt.notValidRationaleCol, axis=1, inplace=True)
        excludedAssessments = pd.DataFrame(self.dfVca).copy()
        excludedAssessments.fillna('', inplace=True)
        excludedAssessments[self.opt.assessmentsIdCol] = excludedAssessments.index
        excludedAssessments = excludedAssessments[(
            (excludedAssessments['Result ' + self.opt.notValidCol] == 'x')
        )]
        excludedAssessments[self.opt.blankCol] = ''
        excludedAssessments[self.opt.excellentCol] = excludedAssessments['Result ' + self.opt.excellentCol]
        excludedAssessments[self.opt.goodCol] = excludedAssessments['Result ' + self.opt.goodCol]
        excludedAssessments[self.opt.notValidCol] = excludedAssessments['Result ' + self.opt.notValidCol]
        excludedAssessments = pd.concat([blanksAssessments, excludedAssessments])

        # create group for final scores
        validAssessmentsRatings = pd.DataFrame(validAssessments).copy()
        validAssessmentsRatings['Rating Given'] = validAssessmentsRatings[
            [self.opt.q0Rating, self.opt.q1Rating, self.opt.q2Rating]
        ].mean(axis=1)
        finalProposals = validAssessmentsRatings.groupby([self.opt.proposalIdCol, self.opt.proposalKeyCol], as_index=False)['Rating Given'].mean().round(2)
        for oProposal in self.proposals:
            if not (finalProposals[self.opt.proposalIdCol] == oProposal['id']).any():
                propToAdd = {}
                propToAdd[self.opt.proposalIdCol] = oProposal['id']
                propToAdd[self.opt.proposalKeyCol] = oProposal['title']
                propToAdd['Rating Given'] = 0
                finalProposals = finalProposals.append(propToAdd, ignore_index=True)

        finalProposals.to_csv('cache/final-proposals.csv')
        # Create list of VCAs
        vcaList = pd.DataFrame(self.vcas)
        vcaList.fillna(0, inplace=True)

        # Save csvs
        self.vcaMerged.to_csv('cache/vca-merged.csv', index=False)
        vcaAggregatedAssessments.to_csv('cache/vca-aggregated.csv')
        aggregatedAssessments.to_csv('cache/aggregated.csv')
        validAssessments.to_csv('cache/valid.csv')
        excludedAssessments.to_csv('cache/excluded.csv')
        # Generate Doc
        spreadsheet = self.gspreadWrapper.createDoc(self.opt.VCAAggregateFileName)

        # Print aggregated assessments
        aggregatedHeadings = [
            self.opt.assessmentsIdCol, self.opt.proposalKeyCol,
            self.opt.challengeCol, self.opt.ideaURLCol, self.opt.assessorCol,
            self.opt.q0Col, self.opt.q0Rating, self.opt.q1Col, self.opt.q1Rating,
            self.opt.q2Col, self.opt.q2Rating, self.opt.proposerMarkCol,
            self.opt.proposersRationaleCol, self.opt.excellentCol,
            self.opt.goodCol, self.opt.notValidCol
        ]
        aggregatedWidths = [
            ('A', 40), ('B:D', 120), ('E', 100), ('F', 300), ('G', 30),
            ('H', 300), ('I', 30), ('J', 300), ('K:L', 30), ('M', 300),
            ('N:P', 30)
        ]
        aggregatedFormats = [
            ('A1:P1', self.utils.headingFormat),
            ('A2:A', self.utils.counterFormat),
            ('G2:G', self.utils.counterFormat),
            ('I2:I', self.utils.counterFormat),
            ('K2:K', self.utils.counterFormat),
            ('L2:L', self.utils.counterFormat),
            ('N2:N', self.utils.counterFormat),
            ('O2:O', self.utils.counterFormat),
            ('P2:P', self.utils.counterFormat),
            ('F2:F', self.utils.noteFormat),
            ('H2:H', self.utils.noteFormat),
            ('J2:J', self.utils.noteFormat),
            ('M2:M', self.utils.noteFormat),
            ('A1:A1', self.utils.verticalHeadingFormat),
            ('G1:G1', self.utils.verticalHeadingFormat),
            ('I1:I1', self.utils.verticalHeadingFormat),
            ('K1:K1', self.utils.verticalHeadingFormat),
            ('L1:L1', self.utils.verticalHeadingFormat),
            ('N1:P1', self.utils.verticalHeadingFormat),
            ('N2:N', self.utils.greenFormat),
            ('O2:O', self.utils.greenFormat),
            ('P2:P', self.utils.yellowFormat)
        ]

        self.gspreadWrapper.createSheetFromDf(
            spreadsheet,
            'Aggregated',
            aggregatedAssessments,
            aggregatedHeadings,
            columnWidths=aggregatedWidths,
            formats=aggregatedFormats
        )

        # Print valid assessments
        validHeadings = [
            self.opt.assessmentsIdCol, self.opt.proposalKeyCol,
            self.opt.proposalIdCol, self.opt.ideaURLCol, self.opt.assessorCol,
            self.opt.q0Col, self.opt.q0Rating, self.opt.q1Col, self.opt.q1Rating,
            self.opt.q2Col, self.opt.q2Rating, self.opt.excellentCol,
            self.opt.goodCol
        ]
        validWidths = [
            ('A', 30), ('B', 120), ('C', 30), ('D', 120), ('E', 100),
            ('F', 300), ('G', 30), ('H', 300), ('I', 30), ('J', 300),
            ('K:M', 30)
        ]
        validFormats = [
            ('A1:M1', self.utils.headingFormat),
            ('A2:A', self.utils.counterFormat),
            ('C2:C', self.utils.counterFormat),
            ('G2:G', self.utils.counterFormat),
            ('I2:I', self.utils.counterFormat),
            ('K2:K', self.utils.counterFormat),
            ('L2:L', self.utils.counterFormat),
            ('M2:M', self.utils.counterFormat),
            ('F:F', self.utils.noteFormat),
            ('H:H', self.utils.noteFormat),
            ('J:J', self.utils.noteFormat),
            ('A1:A1', self.utils.verticalHeadingFormat),
            ('C1:C1', self.utils.verticalHeadingFormat),
            ('G1:G1', self.utils.verticalHeadingFormat),
            ('I1:I1', self.utils.verticalHeadingFormat),
            ('K1:K1', self.utils.verticalHeadingFormat),
            ('L1:M1', self.utils.verticalHeadingFormat),
            ('L2:L', self.utils.greenFormat),
            ('M2:M', self.utils.greenFormat)
        ]

        self.gspreadWrapper.createSheetFromDf(
            spreadsheet,
            'Valid Assessments (excluding Natives)',
            generalValidAssessments,
            validHeadings,
            columnWidths=validWidths,
            formats=validFormats
        )
        for native in validNatives:
            self.gspreadWrapper.createSheetFromDf(
                spreadsheet,
                "Valid Assessments ({})".format(native["title"]),
                native["validAssessments"],
                validHeadings,
                columnWidths=validWidths,
                formats=validFormats
            )


        proposalsWidths = [
            ('A', 300), ('B', 60), ('C', 60)
        ]
        proposalsFormats = [
            ('B:B', self.utils.counterFormat),
            ('C:C', self.utils.counterFormat),
            ('A:A', self.utils.noteFormat),
            ('A1:C1', self.utils.headingFormat),
            ('B1', self.utils.verticalHeadingFormat),
            ('C1', self.utils.verticalHeadingFormat),
        ]

        self.gspreadWrapper.createSheetFromDf(
            spreadsheet,
            'Proposals scores',
            finalProposals,
            [self.opt.proposalKeyCol, self.opt.proposalIdCol, 'Rating Given'],
            columnWidths=proposalsWidths,
            formats=proposalsFormats
        )

        # Print excluded assessments
        excludedHeadings = [
            self.opt.assessmentsIdCol, self.opt.proposalKeyCol,
            self.opt.ideaURLCol, self.opt.assessorCol,
            self.opt.q0Col, self.opt.q0Rating, self.opt.q1Col, self.opt.q1Rating,
            self.opt.q2Col, self.opt.q2Rating, self.opt.blankCol,
            self.opt.notValidCol
        ]
        excludedWidths = [
            ('A', 30), ('B', 120), ('C', 120), ('D', 100),
            ('E', 300), ('F', 30), ('G', 300), ('H', 30), ('I', 300),
            ('J:L', 30)
        ]
        excludedFormats = [
            ('A1:L1', self.utils.headingFormat),
            ('A2:A', self.utils.counterFormat),
            ('F2:F', self.utils.counterFormat),
            ('H2:H', self.utils.counterFormat),
            ('J2:J', self.utils.counterFormat),
            ('K2:K', self.utils.counterFormat),
            ('L2:L', self.utils.counterFormat),
            ('E:E', self.utils.noteFormat),
            ('G:G', self.utils.noteFormat),
            ('I:I', self.utils.noteFormat),
            ('A1:A1', self.utils.verticalHeadingFormat),
            ('F1:F1', self.utils.verticalHeadingFormat),
            ('H1:H1', self.utils.verticalHeadingFormat),
            ('J1:J1', self.utils.verticalHeadingFormat),
            ('K1:K1', self.utils.verticalHeadingFormat),
            ('L1:L1', self.utils.verticalHeadingFormat),
            ('K2:K', self.utils.redFormat),
            ('L2:L', self.utils.yellowFormat)
        ]

        self.gspreadWrapper.createSheetFromDf(
            spreadsheet,
            'Excluded Assessments',
            excludedAssessments,
            excludedHeadings,
            columnWidths=excludedWidths,
            formats=excludedFormats
        )

        vcasWidths = [
            ('A', 100), ('B', 600)
        ]
        vcasFormats = [
            ('A1:C1', self.utils.headingFormat),
        ]

        vcaCols = [self.opt.vcaName, 'vca_link', 'No. of Reviews']
        for nativeChallenge in self.dChallenges:
            reviews_num_col = "No. of Reviews " + nativeChallenge
            vcaCols.append(reviews_num_col)
        self.gspreadWrapper.createSheetFromDf(
            spreadsheet,
            'Veteran Community Advisors',
            vcaList,
            vcaCols,
            columnWidths=vcasWidths,
            formats=vcasFormats
        )

        # Print aggregated assessments
        vcaAggregatedHeadings = [
            self.opt.assessmentsIdCol, self.opt.proposalKeyCol,
            self.opt.ideaURLCol, self.opt.assessorCol,
            self.opt.q0Col, self.opt.q0Rating, self.opt.q1Col, self.opt.q1Rating,
            self.opt.q2Col, self.opt.q2Rating, self.opt.proposerMarkCol,
            self.opt.proposersRationaleCol, self.opt.excellentCol,
            self.opt.goodCol, self.opt.notValidCol, self.opt.noVCAReviewsCol,
            'Result ' + self.opt.excellentCol, 'Result ' + self.opt.goodCol,
            'Result ' + self.opt.notValidCol, self.opt.proposalIdCol

        ]
        vcaAggregatedWidths = [
            ('A', 40), ('B:D', 120), ('E', 300), ('F', 30),
            ('G', 300), ('H', 30), ('I', 300), ('J:K', 30), ('L', 300),
            ('M:T', 30)
        ]
        vcaAggregatedFormats = [
            ('A1:T1', self.utils.headingFormat),
            ('A2:A', self.utils.counterFormat),
            ('F2:F', self.utils.counterFormat),
            ('H2:H', self.utils.counterFormat),
            ('J2:J', self.utils.counterFormat),
            ('K2:K', self.utils.counterFormat),
            ('M2:M', self.utils.counterFormat),
            ('N2:N', self.utils.counterFormat),
            ('O2:O', self.utils.counterFormat),
            ('P2:P', self.utils.counterFormat),
            ('Q2:Q', self.utils.counterFormat),
            ('R2:R', self.utils.counterFormat),
            ('S2:S', self.utils.counterFormat),
            ('E2:E', self.utils.noteFormat),
            ('G2:G', self.utils.noteFormat),
            ('I2:I', self.utils.noteFormat),
            ('L2:L', self.utils.noteFormat),
            ('A1:A1', self.utils.verticalHeadingFormat),
            ('F1:F1', self.utils.verticalHeadingFormat),
            ('H1:H1', self.utils.verticalHeadingFormat),
            ('J1:J1', self.utils.verticalHeadingFormat),
            ('K1:K1', self.utils.verticalHeadingFormat),
            ('M1:S1', self.utils.verticalHeadingFormat),
            ('M2:M', self.utils.greenFormat),
            ('N2:N', self.utils.greenFormat),
            ('Q2:Q', self.utils.greenFormat),
            ('R2:R', self.utils.greenFormat),
            ('O2:O', self.utils.yellowFormat),
            ('S2:S', self.utils.yellowFormat)
        ]

        self.gspreadWrapper.createSheetFromDf(
            spreadsheet,
            'vCA Aggregated',
            vcaAggregatedAssessments,
            vcaAggregatedHeadings,
            columnWidths=vcaAggregatedWidths,
            formats=vcaAggregatedFormats
        )


        print('Aggregated Document created')
        print('Link: {}'.format(spreadsheet.url))

    def calculateOutcome(self, row):
        bad = ''
        good = ''
        excellent = ''
        tot = row[self.opt.noVCAReviewsCol]
        if (tot >= self.opt.minimumVCA):
            if (row[self.opt.excellentCol] > (tot/2)):
                excellent = 'x'
            elif (row[self.opt.notValidCol] >= (tot/2)):
                bad = 'x'
            else:
                good = 'x'
        return (bad, good, excellent)

    def goodFeedback(self, row):
        for col in self.goodColumns:
            if (self.utils.checkIfMarked(row, col) > 0):
                return True
        return False

    def badFeedback(self, row):
        for col in self.badColumns:
            if (self.utils.checkIfMarked(row, col) > 0):
                return True
        return False

    def excellentFeedback(self, row):
        for col in self.excellentColumns:
            if (self.utils.checkIfMarked(row, col) > 0):
                return True
        return False

    def isVCAfeedbackValid(self, row, bad, good, excellent):
        return (sum([bad, good, excellent]) <= 1)

    def filterVCAConficts(self, data, filename):
        toInclude = []
        filename = os.path.basename(filename.replace('\\',os.sep))
        vca = next((item for item in self.vcas if item['vca_file'] == filename), None)
        if (vca):
            for id, row in data.iterrows():
                ass = row.to_dict()
                proposal = next((item for item in self.proposals if item["id"] == ass[self.opt.proposalIdCol]), None)
                if (proposal):
                    # Exclude reviews for self reviews and for challenges
                    # where vCAs are proposers
                    if (
                        (ass[self.opt.assessorCol] != vca['ca_id']) and
                        (proposal["category"] not in vca["campaigns_as_proposers"])
                    ):
                        toInclude.append(row)

            print("Imported file {}".format(filename))
            return pd.DataFrame(toInclude)
        else:
            print("Error importing file {}".format(filename))
            return False



c = createVCAAggregate()
c.createDoc()
