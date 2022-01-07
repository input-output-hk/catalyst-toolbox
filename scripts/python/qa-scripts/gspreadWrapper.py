import gspread
from gspread.models import Cell
from gspread_formatting import *
from gspread.utils import (
    finditem,
    fill_gaps
)

from options import Options
from utils import Utils
import pandas as pd


class GspreadWrapper():
    def __init__(self):
        self.opt = Options()
        self.utils = Utils()
        self.gc = gspread.service_account(filename=self.opt.gSheetAuthFile)

    def loadAssessmentsFile(self):
        self.assessmentsDoc = self.gc.open_by_key(self.opt.originalExportFromIdeascale)
        self.assessmentsSheet = self.assessmentsDoc.worksheet(self.opt.assessmentsSheet)
        self.df = False

    def prepareDataFromExport(self):
        if (self.assessmentsSheet):
            df = pd.DataFrame(self.assessmentsSheet.get_all_records())
            # Assign assessor_id
            df['assessor_id'] = df[self.opt.assessorCol].str.replace('z_assessor_', '')
            # Assign triplet_id
            df[self.opt.tripletIdCol] = df['assessor_id'] + '-' + df[self.opt.proposalIdCol].astype(str)
            # Assign question_id
            df[self.opt.questionIdCol] = df.groupby(self.opt.questionCol).ngroup() + 1
            reviews = []
            # Group assessments per triplet, to obtain the full review
            grouped = df.groupby(self.opt.tripletIdCol)
            for name, group in grouped:
                for criteriaGroup in self.opt.criteriaGroups:
                    criteriaIds = [int(i) for i in list(criteriaGroup.keys())]
                    filtered = group[group[self.opt.questionIdCol].isin(criteriaIds)]
                    if (len(filtered) > 0):
                        # create a new review row, and add notes and ratings
                        # accordingly
                        row = filtered.iloc[0].to_dict()
                        for idx in criteriaGroup:
                            single_filtered = group[group[self.opt.questionIdCol] == int(idx)]
                            if (len(single_filtered) > 0):
                                single = single_filtered.iloc[0].to_dict()
                                row[criteriaGroup[idx] + ' Note'] = single[self.opt.assessmentCol]
                                row[criteriaGroup[idx] + ' Rating'] = single[self.opt.ratingCol]
                        reviews.append(row)
            self.df = pd.DataFrame(reviews)
            # Generate an unique index as a column
            self.df.insert(0, self.opt.assessmentsIdCol, self.df.index + 1)
            self.df.fillna('', inplace=True)
            return self.df
        return False

    def getProposersAggregatedData(self):
        self.proposersDoc = self.gc.open_by_key(self.opt.proposersAggregateFile)
        self.proposersSheet = self.proposersDoc.worksheet(self.opt.assessmentsSheet)
        self.dfProposers = pd.DataFrame(self.proposersSheet.get_all_records())

    def getProposersMasterData(self):
        self.proposersMasterDoc = self.gc.open_by_key(self.opt.proposersMasterFile)
        self.proposersSheet = self.proposersMasterDoc.worksheet(self.opt.assessmentsSheet)
        self.dfMasterProposers = pd.DataFrame(self.proposersSheet.get_all_records())

    def getVCAMasterData(self):
        self.vcaMasterDoc = self.gc.open_by_key(self.opt.VCAMasterFile)
        self.vcaSheet = self.vcaMasterDoc.worksheet(self.opt.assessmentsSheet)
        self.dfVca = pd.DataFrame(self.vcaSheet.get_all_records())

    def getVCAMasterAssessors(self):
        self.vcaDoc = self.gc.open_by_key(self.opt.VCAMasterFile)
        self.vcaSheet = self.vcaDoc.worksheet('Community Advisors')
        self.dfVcaAssessors = pd.DataFrame(self.vcaSheet.get_all_records())

    def createDoc(self, name):
        print('Create new document...')
        spreadsheet = self.gc.create(name)
        spreadsheet.share(
            self.opt.accountEmail,
            perm_type='user',
            role='writer'
        )

        return spreadsheet

    def writeDf(self, worksheet, df, headings=False):
        # Extract the columns already present in dataframe
        existingHeadings = list(df.columns)
        if headings == False:
            headings = existingHeadings
        # Create new Dataframe only with needed columns
        newDf = pd.DataFrame()
        for col in headings:
            if (col in existingHeadings):
                newDf[col] = df[col]
            else:
                newDf[col] = ""

        worksheet.update(
            [newDf.columns.values.tolist()] + newDf.values.tolist()
        )

    def createSheetFromDf(self, spreadsheet, sheetName, df, headings=False, columnWidths=False, formats=False):
        print('Create sheet...')
        worksheet = spreadsheet.add_worksheet(title=sheetName, rows=1, cols=1)
        self.writeDf(worksheet, df, headings)
        if (columnWidths is not False):
            set_column_widths(worksheet, columnWidths)
        if (formats is not False):
            format_cell_ranges(worksheet, formats)

        worksheet.freeze(rows=1)
