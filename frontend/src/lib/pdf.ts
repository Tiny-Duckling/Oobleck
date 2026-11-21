import { jsPDF } from 'jspdf';
import type { QuizDetail } from './api';

const PAGE_MARGIN = 48;
const FOOTER_SPACE = 28;

export function downloadQuizPdf(quiz: QuizDetail): void {
	const doc = new jsPDF({ unit: 'pt', format: 'a4' });
	const pageWidth = doc.internal.pageSize.getWidth();
	const pageHeight = doc.internal.pageSize.getHeight();
	const contentWidth = pageWidth - PAGE_MARGIN * 2;
	let y = PAGE_MARGIN;

	function ensureSpace(required: number): void {
		if (y + required <= pageHeight - PAGE_MARGIN - FOOTER_SPACE) return;
		doc.addPage();
		y = PAGE_MARGIN;
	}

	function writeWrapped(
		text: string,
		options: { indent?: number; lineHeight?: number } = {}
	): void {
		const indent = options.indent ?? 0;
		const lineHeight = options.lineHeight ?? 16;
		const lines = doc.splitTextToSize(text, contentWidth - indent) as string[];

		for (const line of lines) {
			ensureSpace(lineHeight);
			doc.text(line, PAGE_MARGIN + indent, y);
			y += lineHeight;
		}
	}

	doc.setFont('helvetica', 'bold');
	doc.setFontSize(22);
	doc.setTextColor(15, 23, 42);
	writeWrapped(quiz.title, { lineHeight: 27 });
	y += 6;

	if (quiz.description.trim()) {
		doc.setFont('helvetica', 'normal');
		doc.setFontSize(11);
		doc.setTextColor(71, 85, 105);
		writeWrapped(quiz.description, { lineHeight: 16 });
		y += 8;
	}

	doc.setFont('helvetica', 'normal');
	doc.setFontSize(10);
	doc.setTextColor(100, 116, 139);
	writeWrapped('Select one answer for each question.', { lineHeight: 14 });
	y += 18;

	for (const [questionIndex, question] of quiz.questions.entries()) {
		ensureSpace(48);
		doc.setFont('helvetica', 'bold');
		doc.setFontSize(11);
		doc.setTextColor(15, 23, 42);
		writeWrapped(`${questionIndex + 1}. ${question.text}`, { lineHeight: 16 });
		y += 5;

		doc.setFont('helvetica', 'normal');
		doc.setFontSize(10.5);
		doc.setTextColor(30, 41, 59);
		for (const [optionIndex, option] of question.options.entries()) {
			const letter = String.fromCharCode(65 + optionIndex);
			writeWrapped(`${letter}. ${option.text}`, { indent: 14, lineHeight: 15 });
		}
		y += 15;
	}

	const totalPages = doc.getNumberOfPages();
	for (let page = 1; page <= totalPages; page += 1) {
		doc.setPage(page);
		doc.setFont('helvetica', 'normal');
		doc.setFontSize(9);
		doc.setTextColor(100, 116, 139);
		doc.text('Oobleck', PAGE_MARGIN, pageHeight - 24);
		doc.text(`Page ${page} of ${totalPages}`, pageWidth - PAGE_MARGIN, pageHeight - 24, {
			align: 'right'
		});
	}

	const filename = quiz.title
		.toLowerCase()
		.trim()
		.replace(/[^a-z0-9]+/g, '-')
		.replace(/^-|-$/g, '');
	doc.save(`${filename || 'quiz'}.pdf`);
}
