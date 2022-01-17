
from gspreadWrapper import GspreadWrapper
from options import Options
from utils import Utils

from gspread.models import Cell
from gspread_formatting import *
from sklearn.feature_extraction.text import TfidfVectorizer
from sklearn.metrics.pairwise import cosine_similarity

import pandas as pd

from warnings import simplefilter
simplefilter(action='ignore', category=DeprecationWarning)

class CreateSimilarity():
    def __init__(self):
        self.opt = Options()
        self.utils = Utils()
        self.gspreadWrapper = GspreadWrapper()

        print('Load vca data...')
        self.gspreadWrapper.getVCAMasterData()

        self.assessors = {}
        self.similarities = []

        self.similarityMinScore = 0.5

        self.initSimilarity()

    def createDoc(self):

        self.findSimilarity()

        for k in self.assessors:
            self.assessors[k]['similarity_other_assessors'] = ','.join(list(dict.fromkeys(self.assessors[k]['similarity_other_assessors'])))

        self.assessors = list(self.assessors.values())

        dfSimilarities = pd.DataFrame(self.similarities)
        dfAssessors = pd.DataFrame(self.assessors)

        dfSimilarities.to_csv('cache/sim5.csv')
        dfAssessors.to_csv('cache/sim_ass5.csv')

        spreadsheet = self.gspreadWrapper.createDoc('Similarity Analysis')

        self.gspreadWrapper.createSheetFromDf(
            spreadsheet,
            'Assessments',
            dfSimilarities,
            ['id A', 'id B', 'Assessor A', 'Assessor B', 'Note A', 'Note B', 'Similarity Score'],
            columnWidths=[('A:B', 50), ('C:D', 150), ('E:F', 300), ('G', 60)],
            formats=[('G', self.utils.counterFormat),('A1:G1', self.utils.headingFormat)]
        )

        self.gspreadWrapper.createSheetFromDf(
            spreadsheet,
            'CAs',
            dfAssessors,
            ['Assessor', 'similarity_other_assessors', 'similarity_count_others', 'similarity_count_self'],
            columnWidths=[('A:B', 150), ('C:D', 60)],
            formats=[
                ('C:D', self.utils.counterFormat),
                ('A1:D1', self.utils.headingFormat),
                ('C1:D1', self.utils.verticalHeadingFormat),
            ]
        )
        worksheet = spreadsheet.get_worksheet(0)
        spreadsheet.del_worksheet(worksheet)

        print('Link: {}'.format(spreadsheet.url))

    def initSimilarity(self):
        self.vectorize = lambda Text: TfidfVectorizer().fit_transform(Text).toarray()
        self.similarity = lambda doc1, doc2: cosine_similarity([doc1, doc2])

    def findSimilarity(self):
        data = self.gspreadWrapper.dfVca
        criteria = [self.opt.q0Col, self.opt.q1Col, self.opt.q2Col]
        for criterium in criteria:
            notes = list(data[criterium])
            ids = list(data[self.opt.assessmentsIdCol])
            vectors = self.vectorize(notes)
            s_vectors = list(zip(ids, vectors))
            plagiarism_results = set()
            progress = 0
            for assessor_a, text_vector_a in s_vectors:
                print("{} of {}".format(progress, len(s_vectors)))
                new_vectors = s_vectors.copy()
                current_index = s_vectors.index((assessor_a, text_vector_a))
                for assessor_b , text_vector_b in s_vectors:
                    internal_index = s_vectors.index((assessor_b, text_vector_b))
                    if (internal_index != current_index):
                        sim_score = self.similarity(text_vector_a, text_vector_b)[0][1]
                        assessor_pair = sorted((assessor_a, assessor_b))
                        score = (assessor_pair[0], assessor_pair[1], sim_score)
                        plagiarism_results.add(score)
                progress = progress + 1
            for res in plagiarism_results:
                if (res[2] > self.similarityMinScore):
                    ass_0 = data.loc[data[self.opt.assessmentsIdCol] == res[0]][self.opt.assessorCol].item()
                    ass_1 = data.loc[data[self.opt.assessmentsIdCol] == res[1]][self.opt.assessorCol].item()
                    assessment_0 = data.loc[data[self.opt.assessmentsIdCol] == res[0]][criterium].item()
                    assessment_1 = data.loc[data[self.opt.assessmentsIdCol] == res[1]][criterium].item()
                    if (ass_0 not in self.assessors):
                        self.assessors[ass_0] = {
                            'Assessor': ass_0,
                            'similarity_other_assessors': [],
                            'similarity_count_others': 0,
                            'similarity_count_self': 0
                        }
                    if (ass_0 != ass_1):
                        self.assessors[ass_0]['similarity_other_assessors'].append(ass_1)
                        self.assessors[ass_0]['similarity_count_others']  = self.assessors[ass_0]['similarity_count_others'] + 1
                    else:
                        self.assessors[ass_0]['similarity_count_self']  = self.assessors[ass_0]['similarity_count_self'] + 1
                    self.similarities.append({
                        'id A': res[0],
                        'id B': res[1],
                        'Assessor A': ass_0,
                        'Assessor B': ass_1,
                        'Note A': assessment_0,
                        'Note B': assessment_1,
                        'Similarity Score': res[2]
                    })


c = CreateSimilarity()
c.createDoc()
